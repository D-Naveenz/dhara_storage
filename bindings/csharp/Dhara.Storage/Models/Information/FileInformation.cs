using Dhara.Storage.Models.Analysis;

namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents file-specific metadata returned from the native runtime.
/// </summary>
/// <param name="Path">Absolute or normalized file path.</param>
/// <param name="Name">Leaf file name.</param>
/// <param name="IsReadOnly">Whether the file is marked read-only.</param>
/// <param name="IsHidden">Whether the file is marked hidden.</param>
/// <param name="IsSystem">Whether the file is marked as a system entry.</param>
/// <param name="IsTemporary">Whether the file is marked temporary.</param>
/// <param name="IsSymbolicLink">Whether the path is a symbolic link.</param>
/// <param name="LinkTarget">Resolved link target when <paramref name="IsSymbolicLink"/> is <see langword="true"/>.</param>
/// <param name="CreatedAtUtc">Creation time in UTC, when available.</param>
/// <param name="ModifiedAtUtc">Last modification time in UTC, when available.</param>
/// <param name="AccessedAtUtc">Last access time in UTC, when available.</param>
/// <param name="DisplayName">Shell or platform display name for the file.</param>
/// <param name="Size">File size in bytes.</param>
/// <param name="FormattedSize">Human-readable size string produced by the native runtime.</param>
/// <param name="FilenameExtension">Extension from the file name, when present.</param>
/// <param name="Analysis">Optional content-analysis snapshot when requested by the caller.</param>
/// <param name="Icon">Optional OS shell icon pixels when requested by the caller.</param>
/// <param name="ShellDetails">Optional Windows shell display fields when available.</param>
public sealed record FileInformation(
    string Path,
    string Name,
    bool IsReadOnly,
    bool IsHidden,
    bool IsSystem,
    bool IsTemporary,
    bool IsSymbolicLink,
    string? LinkTarget,
    DateTimeOffset? CreatedAtUtc,
    DateTimeOffset? ModifiedAtUtc,
    DateTimeOffset? AccessedAtUtc,
    string DisplayName,
    ulong Size,
    string FormattedSize,
    string? FilenameExtension,
    AnalysisReport? Analysis,
    ShellIcon? Icon,
    ShellDetails? ShellDetails)
    : StorageInformation(Path, Name, IsReadOnly, IsHidden, IsSystem, IsTemporary, IsSymbolicLink, LinkTarget, CreatedAtUtc, ModifiedAtUtc, AccessedAtUtc);
