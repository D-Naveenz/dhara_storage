namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents a listed directory entry.
/// </summary>
/// <param name="Kind">Entry kind string from the native runtime (for example <c>file</c> or <c>directory</c>).</param>
/// <param name="Path">Absolute or normalized path of the entry.</param>
/// <param name="Name">Leaf name of the entry.</param>
public sealed record StorageEntry(
    string Kind,
    string Path,
    string Name)
{
    /// <summary>
    /// Gets a value indicating whether the entry is a directory.
    /// </summary>
    public bool IsDirectory => string.Equals(Kind, "directory", StringComparison.OrdinalIgnoreCase);
}
