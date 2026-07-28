using System.Diagnostics;
using System.IO.Pipes;
using System.Net.Http;
using System.Security.Principal;
using Dhara.Storage.Sd.V1;
using Grpc.Net.Client;

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
    /// Starts <c>dhara-sd</c> listening on <paramref name="fullPipePath"/>.
    /// </summary>
    public static DaemonProcess Start(string fullPipePath, string? exePath = null)
    {
        var exe = exePath ?? ResolveDaemonExe();
        var startInfo = new ProcessStartInfo
        {
            FileName = exe,
            Arguments = $"\"{fullPipePath}\"",
            UseShellExecute = false,
            CreateNoWindow = true,
            // Unread redirected stderr fills and blocks the daemon under info logging.
            RedirectStandardOutput = false,
            RedirectStandardError = false,
        };

        var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException($"Failed to start dhara-sd at '{exe}'.");
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
                _process.WaitForExit(3000);
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

        var local = Path.Combine(AppContext.BaseDirectory, "dhara-sd.exe");
        if (File.Exists(local))
        {
            return local;
        }

        var repoRoot = FindRepoRoot();
        foreach (var config in new[] { "release", "debug" })
        {
            var candidate = Path.Combine(repoRoot, "target", config, "dhara-sd.exe");
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        throw new FileNotFoundException(
            "dhara-sd.exe not found. Build with `cargo build -p dhara-sd` or pack the NuGet sidecar.",
            local);
    }

    internal static string FindRepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null)
        {
            if (File.Exists(Path.Combine(dir.FullName, "dhara.config.toml")))
            {
                return dir.FullName;
            }

            dir = dir.Parent;
        }

        throw new InvalidOperationException("Could not locate repository root (dhara.config.toml).");
    }
}

/// <summary>
/// Creates a gRPC channel over a Windows named pipe for <c>dhara-sd</c>.
/// </summary>
internal static class DaemonChannel
{
    private const int MaxMessageBytes = 16 * 1024 * 1024;

    /// <summary>
    /// Builds an HTTP/2 gRPC channel connected to <paramref name="pipeName"/> (short name, no <c>\\.\pipe\</c> prefix).
    /// </summary>
    public static GrpcChannel CreateNamedPipeChannel(string pipeName)
    {
        var handler = new SocketsHttpHandler
        {
            EnableMultipleHttp2Connections = false,
            ConnectCallback = async (_, cancellationToken) =>
            {
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
}

/// <summary>
/// Process-wide <c>dhara-sd</c> lifecycle and gRPC client.
/// </summary>
public sealed class DharaRuntime : IAsyncDisposable
{
    private static readonly object Gate = new();
    private static DharaRuntime? _current;

    private readonly DaemonProcess _process;
    private readonly GrpcChannel _channel;

    private DharaRuntime(DaemonProcess process, GrpcChannel channel, DharaSd.DharaSdClient client, string pipeName)
    {
        _process = process;
        _channel = channel;
        Client = client;
        PipeName = pipeName;
    }

    /// <summary>Active gRPC client.</summary>
    public DharaSd.DharaSdClient Client { get; }

    /// <summary>Short named-pipe name in use.</summary>
    public string PipeName { get; }

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
        string? pipeName = null,
        string? daemonExePath = null,
        CancellationToken cancellationToken = default)
    {
        if (!OperatingSystem.IsWindows())
        {
            throw new PlatformNotSupportedException("Dhara.Storage daemon transport is Windows-only until UDS lands.");
        }

        lock (Gate)
        {
            if (_current is not null)
            {
                return _current;
            }
        }

        pipeName ??= $"dhara-sd-{Environment.ProcessId}-{Guid.NewGuid():N}";
        var fullPipePath = $@"\\.\pipe\{pipeName}";
        var process = DaemonProcess.Start(fullPipePath, daemonExePath);

        try
        {
            await Task.Delay(200, cancellationToken).ConfigureAwait(false);
            var channel = DaemonChannel.CreateNamedPipeChannel(pipeName);
            var client = new DharaSd.DharaSdClient(channel);
            await WaitForReadyAsync(client, cancellationToken).ConfigureAwait(false);
            await client.HandshakeAsync(
                new HandshakeRequest
                {
                    ParentPid = (uint)Environment.ProcessId,
                    ProtocolVersion = 1,
                },
                cancellationToken: cancellationToken).ConfigureAwait(false);

            var runtime = new DharaRuntime(process, channel, client, pipeName);
            lock (Gate)
            {
                _current ??= runtime;
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

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
    {
        lock (Gate)
        {
            if (ReferenceEquals(_current, this))
            {
                _current = null;
            }
        }

        _channel.Dispose();
        _process.Dispose();
        await Task.CompletedTask.ConfigureAwait(false);
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
