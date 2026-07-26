namespace Dhara.Storage.Models.Watching;

/// <summary>
/// Represents a directory change notification raised by <see cref="StorageDirectory"/>.
/// </summary>
public sealed class StorageChangedEventArgs : EventArgs
{
    /// <summary>
    /// Initializes a new instance of the <see cref="StorageChangedEventArgs"/> class.
    /// </summary>
    /// <param name="path">Current path associated with the change.</param>
    /// <param name="previousPath">Previous path when the change is a relocation; otherwise <see langword="null"/>.</param>
    /// <param name="changeType">High-level change classification.</param>
    /// <param name="observedAt">Timestamp captured by the native watcher.</param>
    public StorageChangedEventArgs(string path, string? previousPath, StorageChangeType changeType, DateTimeOffset observedAt)
    {
        Path = path;
        PreviousPath = previousPath;
        ChangeType = changeType;
        ObservedAt = observedAt;
    }

    /// <summary>
    /// Gets the current path associated with the event.
    /// </summary>
    public string Path { get; }

    /// <summary>
    /// Gets the previous path when the event represents a relocation.
    /// </summary>
    public string? PreviousPath { get; }

    /// <summary>
    /// Gets the high-level change type.
    /// </summary>
    public StorageChangeType ChangeType { get; }

    /// <summary>
    /// Gets the timestamp captured by the native watcher.
    /// </summary>
    public DateTimeOffset ObservedAt { get; }
}
