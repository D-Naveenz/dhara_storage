using System.Runtime.CompilerServices;
using System.Threading.Tasks;

namespace Dhara.Storage;

/// <summary>
/// Awaitable handle for an in-flight storage transfer (copy/move).
/// </summary>
/// <remarks>
/// Work starts as soon as the process is created. Prefer composing with
/// <see cref="Completion"/> / <see cref="GetAwaiter"/> rather than subclassing
/// <see cref="System.Threading.Tasks.Task"/>.
/// </remarks>
public sealed class StorageProcess
{
    private readonly TaskCompletionSource<string?> _completion;
    private readonly CancellationTokenSource _cancellation;

    private StorageProcess(TaskCompletionSource<string?> completion, CancellationTokenSource cancellation)
    {
        _completion = completion;
        _cancellation = cancellation;
    }

    /// <summary>
    /// Gets the awaitable task for this process. Completes with the destination path when reported.
    /// </summary>
    public Task<string?> Completion => _completion.Task;

    /// <summary>
    /// Gets the cancellation token wired into the underlying daemon RPC.
    /// </summary>
    public CancellationToken CancellationToken => _cancellation.Token;

    /// <summary>
    /// Requests cooperative cancellation of the in-flight transfer.
    /// </summary>
    public void Cancel() => _cancellation.Cancel();

    /// <summary>
    /// Returns an awaiter so <c>await process</c> works without subclassing <see cref="System.Threading.Tasks.Task"/>.
    /// </summary>
    public TaskAwaiter<string?> GetAwaiter() => Completion.GetAwaiter();

    /// <summary>
    /// Starts work immediately and returns a process handle completed from <paramref name="work"/>.
    /// </summary>
    /// <param name="work">Async body that performs the transfer and returns an optional destination path.</param>
    /// <param name="cancellationToken">Caller cancellation linked into this process.</param>
    /// <returns>A running <see cref="StorageProcess"/>.</returns>
    internal static StorageProcess Start(
        Func<CancellationToken, Task<string?>> work,
        CancellationToken cancellationToken)
    {
        var linked = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        var completion = new TaskCompletionSource<string?>(TaskCreationOptions.RunContinuationsAsynchronously);
        var process = new StorageProcess(completion, linked);

        _ = RunAsync();
        return process;

        async Task RunAsync()
        {
            try
            {
                var destination = await work(linked.Token).ConfigureAwait(false);
                completion.TrySetResult(destination);
            }
            catch (OperationCanceledException oce)
            {
                completion.TrySetCanceled(oce.CancellationToken);
            }
            catch (Exception ex)
            {
                completion.TrySetException(ex);
            }
        }
    }
}
