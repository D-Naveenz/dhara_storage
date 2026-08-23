namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents directory-specific metadata returned from the native runtime.
/// </summary>
/// <param name="Name">Leaf directory name.</param>
/// <param name="Attributes">Settable filesystem attributes snapshot.</param>
/// <param name="Permissions">Effective permissions snapshot for the current process.</param>
/// <param name="IsSymbolicLink">Whether the path is a symbolic link.</param>
/// <param name="LinkTarget">Resolved link target when <paramref name="IsSymbolicLink"/> is <see langword="true"/>.</param>
/// <param name="IsTemporary">Whether the directory is marked temporary or lives in a temp location.</param>
/// <param name="CreatedAtUtc">Creation time in UTC, when available.</param>
/// <param name="ModifiedAtUtc">Last modification time in UTC, when available.</param>
/// <param name="AccessedAtUtc">Last access time in UTC, when available.</param>
/// <param name="TypeName">Portable directory type label.</param>
/// <param name="Summary">Optional recursive size and entry counts when requested by the caller.</param>
/// <param name="Icon">Optional OS shell icon pixels when requested by the caller.</param>
public sealed record DirectoryMetadata(
    string Name,
    StorageAttributes Attributes,
    StoragePermissions Permissions,
    bool IsSymbolicLink,
    string? LinkTarget,
    bool IsTemporary,
    DateTimeOffset? CreatedAtUtc,
    DateTimeOffset? ModifiedAtUtc,
    DateTimeOffset? AccessedAtUtc,
    string TypeName,
    DirectorySummary? Summary,
    ShellIcon? Icon)
    : StorageMetadata(Name, Attributes, Permissions, IsSymbolicLink, LinkTarget, IsTemporary, CreatedAtUtc, ModifiedAtUtc, AccessedAtUtc);
