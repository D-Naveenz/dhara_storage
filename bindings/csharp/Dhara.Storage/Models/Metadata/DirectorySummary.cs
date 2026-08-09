namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents an optional recursive summary for a directory tree.
/// </summary>
/// <param name="Size">Total size of files discovered under the directory.</param>
/// <param name="FileCount">Number of files discovered under the directory.</param>
/// <param name="DirectoryCount">Number of child directories discovered under the directory.</param>
public sealed record DirectorySummary(
    StorageSize Size,
    ulong FileCount,
    ulong DirectoryCount);
