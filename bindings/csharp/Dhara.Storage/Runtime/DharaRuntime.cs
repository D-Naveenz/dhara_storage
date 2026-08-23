using System.Diagnostics;
using System.IO.Pipes;
using System.Net.Http;
using System.Net.Sockets;
using System.Runtime.InteropServices;
using System.Security.Principal;
using Dhara.Storage.Sd.V1;
using Grpc.Net.Client;
using Microsoft.Win32.SafeHandles;

namespace Dhara.Storage.Runtime;

/// <summary>
/// Starts and stops the <c>dhara-sd</c> sidecar process.
/// </summary>
internal sealed class DaemonProcess : IDisposable
{
    private readonly Process _process;

    private DaemonProcess(Process process) => _process = process;

    /// <summary>OS process id of the running daemon.</summary>
    public int Id => _process.Id;

    /// <summary>
    /// Starts <c>dhara-sd</c> with the platform-specific endpoint argument.
    /// </summary>
    public static DaemonProcess Start(string endpointArgument, string? exePath = null)
    {
        var exe = exePath ?? ResolveDaemonExe();
        var startInfo = new ProcessStartInfo
        {
            FileName = exe,
            Arguments = $"\"{endpointArgument}\"",
            UseShellExecute = false,
            CreateNoWindow = true,
            // Unread redirected stderr fills and blocks the daemon under info logging.
            RedirectStandardOutput = false,
            RedirectStandardError = false,
        };

        var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException($"Failed to start dhara-sd at '{exe}'.");
        DaemonProcessJob.TryAssign(process);
        return new DaemonProcess(process);
    }

    /// <inheritdoc />
    public void Dispose()
    {
        try
        {
            if (!_process.HasExited)
            {
                _process.Kill(entireProcessTree: true);
                if (!_process.WaitForExit(3000))
                {
                    _process.Kill(entireProcessTree: true);
                }
            }
        }
        catch
        {
            // Best-effort shutdown.
        }
        finally
        {
            _process.Dispose();
        }
    }

    internal static string ResolveDaemonExe(string? overridePath = null)
    {
        if (!string.IsNullOrWhiteSpace(overridePath) && File.Exists(overridePath))
        {
            return overridePath;
        }

        var fileName = OperatingSystem.IsWindows() ? "dhara-sd.exe" : "dhara-sd";

        foreach (var candidate in EnumerateDaemonCandidates(fileName))
        {
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        throw new FileNotFoundException(
            $"{fileName} not found. Build with `cargo build -p dhara-sd` or pack the NuGet sidecar.",
            Path.Combine(AppContext.BaseDirectory, fileName));
    }

    private static IEnumerable<string> EnumerateDaemonCandidates(string fileName)
    {
        yield return Path.Combine(AppContext.BaseDirectory, fileName);

        var rid = GetRuntimeIdentifier();
        if (!string.IsNullOrWhiteSpace(rid))
        {
            yield return Path.Combine(AppContext.BaseDirectory, "runtimes", rid, "native", fileName);
        }

        if (TryFindRepoRoot(out var repoRoot))
        {
            foreach (var config in new[] { "release", "debug" })
            {
                yield return Path.Combine(repoRoot, "target", config, fileName);
            }
        }
    }

    private static string? GetRuntimeIdentifier()
    {
        var rid = AppContext.GetData("RUNTIME_IDENTIFIER") as string;
        if (!string.IsNullOrWhiteSpace(rid))
        {
            return rid;
        }

        if (OperatingSystem.IsWindows())
        {
            return Environment.Is64BitProcess
                ? (Environment.GetEnvironmentVariable("PROCESSOR_ARCHITECTURE") is "ARM64" ? "win-arm64" : "win-x64")
                : null;
        }

        if (OperatingSystem.IsLinux())
        {
            return Environment.Is64BitProcess
                ? (RuntimeInformation.ProcessArchitecture == Architecture.Arm64 ? "linux-arm64" : "linux-x64")
                : null;
        }

        if (OperatingSystem.IsMacOS() && Environment.Is64BitProcess)
        {
            return "osx-arm64";
        }

        return null;
    }

    internal static bool TryFindRepoRoot(out string repoRoot)
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null)
        {
            if (File.Exists(Path.Combine(dir.FullName, "dhara.config.toml")))
            {
                repoRoot = dir.FullName;
                return true;
            }

            dir = dir.Parent;
        }

        repoRoot = string.Empty;
        return false;
    }

    internal static string FindRepoRoot()
    {
        if (TryFindRepoRoot(out var repoRoot))
        {
            return repoRoot;
        }

        throw new InvalidOperationException("Could not locate repository root (dhara.config.toml).");
    }
}

/// <summary>
/// Creates a gRPC channel for <c>dhara-sd</c> over named pipes (Windows) or UDS (Linux/macOS).
/// </summary>
internal static class DaemonChannel
{
    private const int MaxMessageBytes = 16 * 1024 * 1024;

    /// <summary>
    /// Builds an HTTP/2 gRPC channel connected to the daemon control endpoint.
    /// </summary>
    public static GrpcChannel Create(string controlEndpoint)
    {
        var handler = new SocketsHttpHandler
        {
            EnableMultipleHttp2Connections = false,
            ConnectCallback = async (_, cancellationToken) =>
            {
                if (OperatingSystem.IsWindows())
                {
                    return await ConnectNamedPipeAsync(controlEndpoint, cancellationToken).ConfigureAwait(false);
                }

                return await ConnectUnixSocketAsync(controlEndpoint, cancellationToken).ConfigureAwait(false);
            },
        };

        return GrpcChannel.ForAddress(
            "http://localhost",
            new GrpcChannelOptions
            {
                HttpHandler = handler,
                DisposeHttpClient = true,
                MaxReceiveMessageSize = MaxMessageBytes,
                MaxSendMessageSize = MaxMessageBytes,
            });
    }

    private static async Task<Stream> ConnectNamedPipeAsync(string fullPipePath, CancellationToken cancellationToken)
    {
        var pipeName = fullPipePath;
        if (pipeName.StartsWith(@"\\.\pipe\", StringComparison.Ordinal))
        {
            pipeName = pipeName[9..];
        }

        var pipe = new NamedPipeClientStream(
            serverName: ".",
            pipeName: pipeName,
            direction: PipeDirection.InOut,
            options: PipeOptions.WriteThrough | PipeOptions.Asynchronous,
            impersonationLevel: TokenImpersonationLevel.Anonymous);

        try
        {
            await pipe.ConnectAsync(5000, cancellationToken).ConfigureAwait(false);
            return pipe;
        }
        catch
        {
            await pipe.DisposeAsync().ConfigureAwait(false);
            throw;
        }
    }

    private static async Task<Stream> ConnectUnixSocketAsync(string socketPath, CancellationToken cancellationToken)
    {
        var socket = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
        try
        {
            await socket.ConnectAsync(new UnixDomainSocketEndPoint(socketPath), cancellationToken)
                .ConfigureAwait(false);
            return new NetworkStream(socket, ownsSocket: true);
        }
        catch
        {
            socket.Dispose();
            throw;
        }
    }
}

/// <summary>
/// Process-wide <c>dhara-sd</c> lifecycle and gRPC client.
/// </summary>
public sealed class DharaRuntime : IAsyncDisposable
{
    private static readonly object Gate = new();
    private static DharaRuntime? _current;
    private static int _processExitHookRegistered;

    private readonly DaemonProcess _process;
    private readonly GrpcChannel _channel;
    private readonly Socket? _fdPassSocket;

    private DharaRuntime(
        DaemonProcess process,
        GrpcChannel channel,
        DharaSd.DharaSdClient client,
        string controlEndpoint,
        string? fdPassEndpoint,
        Socket? fdPassSocket)
    {
        _process = process;
        _channel = channel;
        Client = client;
        ControlEndpoint = controlEndpoint;
        FdPassEndpoint = fdPassEndpoint;
        _fdPassSocket = fdPassSocket;
    }

    /// <summary>Active gRPC client.</summary>
    public DharaSd.DharaSdClient Client { get; }

    /// <summary>Control-plane endpoint path (named pipe or UDS).</summary>
    public string ControlEndpoint { get; }

    /// <summary>Unix FD-pass socket path, when applicable.</summary>
    public string? FdPassEndpoint { get; }

    /// <summary>Connected FD-pass socket on Unix.</summary>
    internal Socket? FdPassSocket => _fdPassSocket;

    /// <summary>Daemon process id.</summary>
    public int DaemonProcessId => _process.Id;

    /// <summary>Currently started runtime, if any.</summary>
    public static DharaRuntime? Current
    {
        get
        {
            lock (Gate)
            {
                return _current;
            }
        }
    }

    /// <summary>
    /// Starts the daemon and completes handshake. Idempotent if already started.
    /// </summary>
    public static async Task<DharaRuntime> StartAsync(
        string? endpoint = null,
        string? daemonExePath = null,
        CancellationToken cancellationToken = default)
    {
        lock (Gate)
        {
            if (_current is not null)
            {
                return _current;
            }
        }

        var endpointArgument = endpoint ?? CreateDefaultEndpointArgument();
        var process = DaemonProcess.Start(endpointArgument, daemonExePath);

        try
        {
            await Task.Delay(200, cancellationToken).ConfigureAwait(false);
            var controlEndpoint = await ResolveControlEndpointAsync(endpointArgument, cancellationToken)
                .ConfigureAwait(false);
            var channel = DaemonChannel.Create(controlEndpoint);
            var client = new DharaSd.DharaSdClient(channel);
            await WaitForReadyAsync(client, cancellationToken).ConfigureAwait(false);

            var handshake = await client.HandshakeAsync(
                new HandshakeRequest
                {
                    ParentPid = (uint)Environment.ProcessId,
                    ProtocolVersion = 1,
                },
                cancellationToken: cancellationToken).ConfigureAwait(false);

            Socket? fdPassSocket = null;
            if (!OperatingSystem.IsWindows() && !string.IsNullOrWhiteSpace(handshake.FdPassEndpoint))
            {
                fdPassSocket = UnixFdPass.Connect(handshake.FdPassEndpoint);
            }

            var runtime = new DharaRuntime(
                process,
                channel,
                client,
                handshake.PipeName,
                string.IsNullOrWhiteSpace(handshake.FdPassEndpoint) ? null : handshake.FdPassEndpoint,
                fdPassSocket);

            lock (Gate)
            {
                _current ??= runtime;
                RegisterProcessExitHookIfNeeded();
                return _current;
            }
        }
        catch
        {
            process.Dispose();
            throw;
        }
    }

    /// <summary>Returns the started runtime, starting it if needed.</summary>
    public static DharaRuntime EnsureStarted() =>
        StartAsync().GetAwaiter().GetResult();

    /// <summary>Receives a duplicated read/write handle from the daemon data plane.</summary>
    internal static SafeFileHandle ReceiveDataPlaneHandle()
    {
        if (OperatingSystem.IsWindows())
        {
            throw new InvalidOperationException("ReceiveDataPlaneHandle is only used on Unix.");
        }

        var runtime = Current ?? throw new InvalidOperationException("dhara-sd has not been started.");
        var socket = runtime.FdPassSocket
            ?? throw new InvalidOperationException("The daemon FD-pass socket is not connected.");
        return UnixFdPass.Receive(socket);
    }

    /// <summary>Best-effort synchronous shutdown for process exit (limited runtime budget).</summary>
    internal static void StopForProcessExit()
    {
        DharaRuntime? runtime;
        lock (Gate)
        {
            runtime = _current;
            _current = null;
        }

        if (runtime is null)
        {
            return;
        }

        try
        {
            runtime._process.Dispose();
        }
        catch
        {
            // Process exit path — best effort only.
        }
    }

    private static void RegisterProcessExitHookIfNeeded()
    {
        if (Interlocked.Exchange(ref _processExitHookRegistered, 1) != 0)
        {
            return;
        }

        AppDomain.CurrentDomain.ProcessExit += static (_, _) => StopForProcessExit();
    }

    /// <inheritdoc />
    public ValueTask DisposeAsync()
    {
        lock (Gate)
        {
            if (ReferenceEquals(_current, this))
            {
                _current = null;
            }
        }

        _fdPassSocket?.Dispose();
        _channel.Dispose();
        _process.Dispose();
        return ValueTask.CompletedTask;
    }

    private static string CreateDefaultEndpointArgument()
    {
        if (OperatingSystem.IsWindows())
        {
            return $@"\\.\pipe\dhara-sd-{Environment.ProcessId}-{Guid.NewGuid():N}";
        }

        return Path.Combine(
            Path.GetTempPath(),
            $"dhara-sd-{Environment.ProcessId}-{Guid.NewGuid():N}");
    }

    private static async Task<string> ResolveControlEndpointAsync(
        string endpointArgument,
        CancellationToken cancellationToken)
    {
        if (OperatingSystem.IsWindows())
        {
            return endpointArgument;
        }

        var grpcPath = Path.Combine(endpointArgument, "grpc.sock");
        var deadline = DateTime.UtcNow.AddSeconds(10);
        while (DateTime.UtcNow < deadline)
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (File.Exists(grpcPath))
            {
                return grpcPath;
            }

            await Task.Delay(100, cancellationToken).ConfigureAwait(false);
        }

        throw new TimeoutException($"Timed out waiting for daemon control socket at '{grpcPath}'.");
    }

    private static async Task WaitForReadyAsync(DharaSd.DharaSdClient client, CancellationToken cancellationToken)
    {
        var deadline = DateTime.UtcNow.AddSeconds(10);
        Exception? last = null;
        while (DateTime.UtcNow < deadline)
        {
            cancellationToken.ThrowIfCancellationRequested();
            try
            {
                await client.PingAsync(new PingRequest(), cancellationToken: cancellationToken).ConfigureAwait(false);
                return;
            }
            catch (Exception ex)
            {
                last = ex;
                await Task.Delay(100, cancellationToken).ConfigureAwait(false);
            }
        }

        throw new TimeoutException("Timed out waiting for dhara-sd Ping.", last);
    }
}
