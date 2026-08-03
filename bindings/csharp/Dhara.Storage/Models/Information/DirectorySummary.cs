namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents an optional recursive summary for a directory tree.
/// </summary>
/// <param name="TotalSize">Total size of files under the directory, in bytes.</param>
/// <param name="FileCount">Number of files discovered under the directory.</param>
/// <param name="DirectoryCount">Number of child directories discovered under the directory.</param>
/// <param name="FormattedSize">Human-readable rendering of <paramref name="TotalSize"/>.</param>
public sealed record DirectorySummary(
    ulong TotalSize,
    ulong FileCount,
    ulong DirectoryCount,
    string FormattedSize);
