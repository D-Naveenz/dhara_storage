namespace Dhara.Storage.Models.Watching;

/// <summary>
/// Configures directory watching behavior for <see cref="StorageDirectory"/>.
/// </summary>
public sealed class StorageWatchOptions
{
    /// <summary>
    /// Gets or sets a value indicating whether child directories should be watched recursively.
    /// </summary>
    public bool Recursive { get; init; } = true;

    /// <summary>
    /// Gets or sets the debounce window forwarded to the native watcher.
    /// </summary>
    public TimeSpan DebounceWindow { get; init; } = TimeSpan.FromMilliseconds(500);

    /// <summary>
    /// Gets or sets the receive timeout used by the managed watch loop.
    /// </summary>
    public TimeSpan ReceiveTimeout { get; init; } = TimeSpan.FromMilliseconds(250);
}
