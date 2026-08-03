using Dhara.Storage.Core;
using Dhara.Storage.Sd.V1;
using Grpc.Core;
using Microsoft.Extensions.Logging;

namespace Dhara.Storage.Runtime;

/// <summary>
/// Consumes the daemon <c>StreamLogs</c> gRPC stream and forwards records into the host logger.
/// </summary>
internal sealed class LogStreamHost : IAsyncDisposable
{
    private readonly CancellationTokenSource _cancellation = new();
    private readonly Task _loop;

    private LogStreamHost(DharaSd.DharaSdClient client, LogLevel minLevel)
    {
        _loop = RunAsync(client, minLevel, _cancellation.Token);
    }

    /// <summary>Starts the background log stream for the active daemon client.</summary>
    internal static LogStreamHost Start(DharaSd.DharaSdClient client, LogLevel minLevel = LogLevel.Information) =>
        new(client, minLevel);

    /// <inheritdoc />
    public async ValueTask DisposeAsync()
    {
        await _cancellation.CancelAsync().ConfigureAwait(false);
        try
        {
            await _loop.ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
        }
    }

    private static async Task RunAsync(
        DharaSd.DharaSdClient client,
        LogLevel minLevel,
        CancellationToken cancellationToken)
    {
        var minRank = ToMinLevel(minLevel);

        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                using var call = client.StreamLogs(
                    new StreamLogsRequest { MinLevel = minRank },
                    cancellationToken: cancellationToken);

                await foreach (var record in call.ResponseStream.ReadAllAsync(cancellationToken)
                                   .ConfigureAwait(false))
                {
                    DharaStorageLogBridge.LogManaged(
                        ParseLevel(record.Level),
                        record.Target,
                        record.Message);
                }
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                break;
            }
            catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
            {
                break;
            }
            catch (RpcException)
            {
                await Task.Delay(250, cancellationToken).ConfigureAwait(false);
            }
        }
    }

    private static uint ToMinLevel(LogLevel level) => level switch
    {
        LogLevel.Critical or LogLevel.Error => 1,
        LogLevel.Warning => 2,
        LogLevel.Information => 3,
        LogLevel.Debug => 4,
        LogLevel.Trace => 5,
        _ => 3,
    };

    private static LogLevel ParseLevel(string level) => level switch
    {
        "ERROR" => LogLevel.Error,
        "WARN" => LogLevel.Warning,
        "INFO" => LogLevel.Information,
        "DEBUG" => LogLevel.Debug,
        "TRACE" => LogLevel.Trace,
        _ => LogLevel.Information,
    };
}
