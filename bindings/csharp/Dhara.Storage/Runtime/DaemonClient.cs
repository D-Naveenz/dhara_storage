using Dhara.Storage.Core;
using Dhara.Storage.Exceptions;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Sd.V1;
using Grpc.Core;
using Microsoft.Extensions.Logging;

namespace Dhara.Storage.Runtime;

/// <summary>
/// Thin call wrapper around the active <c>dhara-sd</c> gRPC client used by the public
/// storage wrappers.
/// </summary>
/// <remarks>Starts the daemon on first use via <see cref="DharaRuntime.EnsureStarted"/> and
/// translates <see cref="RpcException"/> failures into <see cref="DharaStorageException"/> so
/// callers observe the same exception type the previous FFI boundary produced. Operational logs
/// from the daemon arrive on the parallel <c>StreamLogs</c> gRPC stream started by
/// <see cref="DharaRuntime"/>.</remarks>
internal static class DaemonClient
{
    private const string LogCategory = "Dhara.Storage.DaemonClient";

    /// <summary>Gets the active gRPC client, starting <c>dhara-sd</c> if it has not been started yet.</summary>
    internal static DharaSd.DharaSdClient Client => DharaRuntime.EnsureStarted().Client;

    /// <summary>Invokes a synchronous unary RPC and maps transport failures to <see cref="DharaStorageException"/>.</summary>
    internal static TResponse Call<TResponse>(
        Func<DharaSd.DharaSdClient, CallOptions, TResponse> call,
        string? path = null,
        string? operation = null)
    {
        var label = operation ?? "daemon RPC";
        try
        {
            var response = call(Client, new CallOptions());
            DharaStorageLogBridge.LogManaged(LogLevel.Trace, LogCategory, $"{label} completed successfully.");
            return response;
        }
        catch (RpcException ex)
        {
            DharaStorageLogBridge.LogManaged(LogLevel.Error, LogCategory, $"{label} failed: {ex.Status.Detail}", ex);
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
        var label = operation ?? "daemon RPC";
        DharaStorageLogBridge.LogManaged(LogLevel.Trace, LogCategory, $"Invoking {label}.");
        try
        {
            using var asyncCall = call(Client, new CallOptions(cancellationToken: cancellationToken));
            var response = await asyncCall.ResponseAsync.ConfigureAwait(false);
            DharaStorageLogBridge.LogManaged(LogLevel.Trace, LogCategory, $"{label} completed successfully.");
            return response;
        }
        catch (RpcException ex)
        {
            DharaStorageLogBridge.LogManaged(LogLevel.Error, LogCategory, $"{label} failed: {ex.Status.Detail}", ex);
            throw ToStorageException(ex, path, operation);
        }
    }

    /// <summary>
    /// Consumes a <c>CopyFile</c> / <c>CopyDirectory</c> progress stream, reporting each update and
    /// returning the final destination path once the daemon reports completion.
    /// </summary>
    internal static async Task<string> ConsumeCopyProgressAsync(
        Func<DharaSd.DharaSdClient, CallOptions, AsyncServerStreamingCall<CopyFileProgress>> call,
        IProgress<StorageProgress>? progress,
        string? path,
        string? operation,
        CancellationToken cancellationToken)
    {
        var label = operation ?? "daemon copy RPC";
        DharaStorageLogBridge.LogManaged(LogLevel.Trace, LogCategory, $"Waiting for {label} to complete.");
        try
        {
            using var streamingCall = call(Client, new CallOptions(cancellationToken: cancellationToken));
            string? destination = null;
            string? errorMessage = null;

            while (await streamingCall.ResponseStream.MoveNext(cancellationToken).ConfigureAwait(false))
            {
                var update = streamingCall.ResponseStream.Current;
                if (update.Completed)
                {
                    destination = update.HasDestination ? update.Destination : null;
                    errorMessage = update.HasErrorMessage ? update.ErrorMessage : null;
                    break;
                }

                progress?.Report(DaemonModelFactory.ToProgress(update));
            }

            if (errorMessage is not null)
            {
                DharaStorageLogBridge.LogManaged(LogLevel.Error, LogCategory, $"{label} failed: {errorMessage}");
                throw new DharaStorageException(errorMessage, "Internal", path, operation);
            }

            if (destination is null)
            {
                throw new DharaStorageException("The copy operation completed without reporting a destination path.", "Internal", path, operation);
            }

            DharaStorageLogBridge.LogManaged(LogLevel.Trace, LogCategory, $"{label} completed successfully.");
            return destination;
        }
        catch (OperationCanceledException)
        {
            DharaStorageLogBridge.LogManaged(LogLevel.Warning, LogCategory, $"{label} was cancelled.");
            throw;
        }
        catch (RpcException ex)
        {
            DharaStorageLogBridge.LogManaged(LogLevel.Error, LogCategory, $"{label} failed: {ex.Status.Detail}", ex);
            throw ToStorageException(ex, path, operation);
        }
    }

    /// <summary>Wraps a transport failure as a <see cref="DharaStorageException"/> carrying the gRPC status code.</summary>
    internal static DharaStorageException ToStorageException(RpcException ex, string? path, string? operation) =>
        new(string.IsNullOrWhiteSpace(ex.Status.Detail) ? ex.Message : ex.Status.Detail, ex.StatusCode.ToString(), path, operation);
}
