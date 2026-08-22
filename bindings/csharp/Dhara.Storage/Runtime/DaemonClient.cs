using Dhara.Storage.Core;
using Dhara.Storage.Exceptions;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Sd.V1;
using Grpc.Core;

namespace Dhara.Storage.Runtime;

/// <summary>
/// Thin call wrapper around the active <c>dhara-sd</c> gRPC client used by the public
/// storage wrappers.
/// </summary>
/// <remarks>Starts the daemon on first use via <see cref="DharaRuntime.EnsureStarted"/> and
/// translates <see cref="RpcException"/> failures into <see cref="DharaStorageException"/> so
/// callers observe the same exception type the previous FFI boundary produced.</remarks>
internal static class DaemonClient
{
    /// <summary>Gets the active gRPC client, starting <c>dhara-sd</c> if it has not been started yet.</summary>
    internal static DharaSd.DharaSdClient Client => DharaRuntime.EnsureStarted().Client;

    /// <summary>Invokes a synchronous unary RPC and maps transport failures to <see cref="DharaStorageException"/>.</summary>
    internal static TResponse Call<TResponse>(
        Func<DharaSd.DharaSdClient, CallOptions, TResponse> call,
        string? path = null,
        string? operation = null)
    {
        try
        {
            return call(Client, new CallOptions());
        }
        catch (RpcException ex)
        {
            throw ToStorageException(ex, path, operation);
        }
    }

    /// <summary>Invokes an asynchronous unary RPC and maps transport failures to <see cref="DharaStorageException"/>.</summary>
    internal static async Task<TResponse> CallAsync<TResponse>(
        Func<DharaSd.DharaSdClient, CallOptions, AsyncUnaryCall<TResponse>> call,
        CancellationToken cancellationToken,
        string? path = null,
        string? operation = null)
    {
        try
        {
            using var asyncCall = call(Client, new CallOptions(cancellationToken: cancellationToken));
            return await asyncCall.ResponseAsync.ConfigureAwait(false);
        }
        catch (RpcException ex)
        {
            throw ToStorageException(ex, path, operation);
        }
    }

    /// <summary>
    /// Starts a copy stream immediately and returns a <see cref="StorageProcess"/> completed from terminal events.
    /// </summary>
    internal static StorageProcess StartCopyProcess(
        Func<DharaSd.DharaSdClient, CallOptions, AsyncServerStreamingCall<StorageProcessEventMessage>> call,
        IProgress<StorageProcessEvent>? progress,
        string? path,
        string? operation,
        CancellationToken cancellationToken) =>
        StorageProcess.Start(
            ct => ConsumeCopyProgressAsync(call, progress, path, operation, ct),
            cancellationToken);

    /// <summary>
    /// Consumes a <c>CopyFile</c> / <c>CopyDirectory</c> progress stream, reporting each update and
    /// returning the final destination path once the daemon reports completion.
    /// </summary>
    internal static async Task<string?> ConsumeCopyProgressAsync(
        Func<DharaSd.DharaSdClient, CallOptions, AsyncServerStreamingCall<StorageProcessEventMessage>> call,
        IProgress<StorageProcessEvent>? progress,
        string? path,
        string? operation,
        CancellationToken cancellationToken)
    {
        var label = operation ?? "daemon copy RPC";
        try
        {
            using var streamingCall = call(Client, new CallOptions(cancellationToken: cancellationToken));
            string? destination = null;
            string? errorMessage = null;
            string? cancelledOperation = null;

            while (await streamingCall.ResponseStream.MoveNext(cancellationToken).ConfigureAwait(false))
            {
                var update = streamingCall.ResponseStream.Current;
                var processEvent = DaemonModelFactory.ToProcessEvent(update);
                if (processEvent is null)
                {
                    continue;
                }

                progress?.Report(processEvent);

                switch (processEvent)
                {
                    case StorageProcessCompleted completed:
                        destination = completed.Destination;
                        goto Done;
                    case StorageProcessFailed failed:
                        errorMessage = failed.Message;
                        goto Done;
                    case StorageProcessCancelled cancelled:
                        cancelledOperation = cancelled.Operation;
                        goto Done;
                }
            }

        Done:
            if (cancelledOperation is not null)
            {
                throw new OperationCanceledException($"{label} was cancelled ({cancelledOperation}).");
            }

            if (errorMessage is not null)
            {
                throw new DharaStorageException(errorMessage, "Internal", path, operation);
            }

            if (destination is null)
            {
                throw new DharaStorageException("The copy operation completed without reporting a destination path.", "Internal", path, operation);
            }

            return destination;
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (RpcException ex)
        {
            throw ToStorageException(ex, path, operation);
        }
    }

    /// <summary>Wraps a transport failure as a <see cref="DharaStorageException"/> carrying the gRPC status code.</summary>
    internal static DharaStorageException ToStorageException(RpcException ex, string? path, string? operation) =>
        new(string.IsNullOrWhiteSpace(ex.Status.Detail) ? ex.Message : ex.Status.Detail, ex.StatusCode.ToString(), path, operation);
}
