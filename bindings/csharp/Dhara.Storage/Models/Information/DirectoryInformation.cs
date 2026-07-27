namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents directory-specific metadata returned from the native runtime.
/// </summary>
/// <param name="Path">Absolute or normalized directory path.</param>
/// <param name="Name">Leaf directory name.</param>
/// <param name="IsReadOnly">Whether the directory is marked read-only.</param>
/// <param name="IsHidden">Whether the directory is marked hidden.</param>
/// <param name="IsSystem">Whether the directory is marked as a system entry.</param>
/// <param name="IsTemporary">Whether the directory is marked temporary.</param>
/// <param name="IsSymbolicLink">Whether the path is a symbolic link.</param>
/// <param name="LinkTarget">Resolved link target when <paramref name="IsSymbolicLink"/> is <see langword="true"/>.</param>
/// <param name="CreatedAtUtc">Creation time in UTC, when available.</param>
/// <param name="ModifiedAtUtc">Last modification time in UTC, when available.</param>
/// <param name="AccessedAtUtc">Last access time in UTC, when available.</param>
/// <param name="DisplayName">Shell or platform display name for the directory.</param>
/// <param name="Summary">Optional recursive size and entry counts when requested by the caller.</param>
/// <param name="Icon">Optional OS shell icon pixels when requested by the caller.</param>
/// <param name="ShellDetails">Optional Windows shell display fields when available.</param>
public sealed record DirectoryInformation(
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
    DirectorySummary? Summary,
    ShellIcon? Icon,
    ShellDetails? ShellDetails)
    : StorageInformation(Path, Name, IsReadOnly, IsHidden, IsSystem, IsTemporary, IsSymbolicLink, LinkTarget, CreatedAtUtc, ModifiedAtUtc, AccessedAtUtc);
