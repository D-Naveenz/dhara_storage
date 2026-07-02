namespace Dhara.Storage;

/// <summary>
/// Reports progress updates immediately on the calling thread.
/// </summary>
/// <typeparam name="T">The progress value type.</typeparam>
/// <remarks>Unlike <see cref="Progress{T}"/>, this implementation does not capture a
/// <see cref="SynchronizationContext"/> and does not dispatch callbacks asynchronously.
/// It is useful when callers want deterministic inline progress delivery, such as console
/// apps, background services, or tests.</remarks>
public sealed class SynchronousProgress<T> : IProgress<T>
{
    private readonly Action<T> _handler;

    /// <summary>
    /// Initializes a new instance of the <see cref="SynchronousProgress{T}"/> class.
    /// </summary>
    /// <param name="handler">The delegate to invoke synchronously for each reported value.</param>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="handler"/> is <see langword="null"/>.</exception>
    public SynchronousProgress(Action<T> handler)
    {
        ArgumentNullException.ThrowIfNull(handler);
        _handler = handler;
    }

    /// <summary>
    /// Reports a progress value immediately to the configured handler.
    /// </summary>
    /// <param name="value">The progress value to forward.</param>
    public void Report(T value) => _handler(value);
}
