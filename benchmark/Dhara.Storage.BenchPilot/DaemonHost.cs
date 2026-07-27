using System.Diagnostics;
using System.IO.Pipes;
using System.Net.Http;
using System.Security.Principal;
using Dhara.Storage.Pilot.V1;
using Grpc.Net.Client;

namespace Dhara.Storage.BenchPilot;

/// <summary>
/// Owns the lifecycle of the Windows named-pipe pilot daemon and a ready gRPC client.
/// </summary>
internal sealed class DaemonHost : IAsyncDisposable
{
    public const string DefaultPipeName = @"dhara-storage-pilot";

    /// <summary>Upper bound for unary payloads in the pilot suite (covers 1MB reads/writes).</summary>
    private const int MaxMessageBytes = 16 * 1024 * 1024;

    private readonly Process _process;
    private readonly GrpcChannel _channel;

    private DaemonHost(Process process, GrpcChannel channel, DharaPilot.DharaPilotClient client, string pipeName)
    {
        _process = process;
        _channel = channel;
        Client = client;
        PipeName = pipeName;
    }

    public DharaPilot.DharaPilotClient Client { get; }

    public string PipeName { get; }

    public int DaemonProcessId => _process.Id;

    public static async Task<DaemonHost> StartAsync(string? pipeName = null, CancellationToken cancellationToken = default)
    {
        pipeName ??= $"{DefaultPipeName}-{Environment.ProcessId}-{Guid.NewGuid():N}";
        var fullPipePath = $@"\\.\pipe\{pipeName}";
        var exe = ResolveDaemonExe();

        var startInfo = new ProcessStartInfo
        {
            FileName = exe,
            Arguments = $"\"{fullPipePath}\"",
            UseShellExecute = false,
            CreateNoWindow = true,
            // Do not redirect stdio: an unread stderr pipe fills and blocks the daemon
            // (AnalyzePath logs enough to hang after a handful of RPCs).
            RedirectStandardOutput = false,
            RedirectStandardError = false,
        };

        var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException($"Failed to start daemon at '{exe}'.");

        try
        {
            // Give the named-pipe server a moment to create the first instance.
            await Task.Delay(200, cancellationToken).ConfigureAwait(false);

            var channel = CreateNamedPipeChannel(pipeName);
            var client = new DharaPilot.DharaPilotClient(channel);

            await WaitForReadyAsync(client, cancellationToken).ConfigureAwait(false);
            await client.HandshakeAsync(
                new HandshakeRequest
                {
                    ParentPid = (uint)Environment.ProcessId,
                    ProtocolVersion = 1,
                },
                cancellationToken: cancellationToken).ConfigureAwait(false);

            return new DaemonHost(process, channel, client, pipeName);
        }
        catch
        {
            TryKill(process);
            throw;
        }
    }

    public async ValueTask DisposeAsync()
    {
        _channel.Dispose();
        TryKill(_process);
        await Task.CompletedTask.ConfigureAwait(false);
    }

    private static GrpcChannel CreateNamedPipeChannel(string pipeName)
    {
        var handler = new SocketsHttpHandler
        {
            // One pipe connection; HTTP/2 multiplexes RPCs. Extra connects need spare
            // daemon listeners and historically hung BDN under iterative load.
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

    private static async Task WaitForReadyAsync(DharaPilot.DharaPilotClient client, CancellationToken cancellationToken)
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

        throw new TimeoutException("Timed out waiting for pilot daemon Ping.", last);
    }

    private static string ResolveDaemonExe()
    {
        var local = Path.Combine(AppContext.BaseDirectory, "dhara-storage-daemon-pilot.exe");
        if (File.Exists(local))
        {
            return local;
        }

        var repoRoot = FindRepoRoot();
        foreach (var config in new[] { "release", "debug" })
        {
            var candidate = Path.Combine(repoRoot, "target", config, "dhara-storage-daemon-pilot.exe");
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        throw new FileNotFoundException(
            "dhara-storage-daemon-pilot.exe not found. Build the pilot daemon first.",
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

    private static void TryKill(Process process)
    {
        try
        {
            if (!process.HasExited)
            {
                process.Kill(entireProcessTree: true);
                process.WaitForExit(3000);
            }
        }
        catch
        {
            // Best-effort cleanup for the pilot harness.
        }
        finally
        {
            process.Dispose();
        }
    }
}
