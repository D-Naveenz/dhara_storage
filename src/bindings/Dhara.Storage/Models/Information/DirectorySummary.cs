namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents an optional recursive summary for a directory tree.
/// </summary>
public sealed record DirectorySummary(
    ulong TotalSize,
    ulong FileCount,
    ulong DirectoryCount,
    string FormattedSize);
