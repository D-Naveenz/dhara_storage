namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents a listed directory entry.
/// </summary>
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
