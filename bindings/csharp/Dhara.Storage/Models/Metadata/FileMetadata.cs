using Dhara.Storage.Models.Analysis;

namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents file-specific metadata returned from the native runtime.
/// </summary>
/// <param name="Name">Leaf file name.</param>
/// <param name="DisplayName">Shell display name when available; otherwise a name-based fallback.</param>
/// <param name="Attributes">Settable filesystem attributes snapshot.</param>
/// <param name="Permissions">Effective permissions snapshot for the current process.</param>
/// <param name="IsSymbolicLink">Whether the path is a symbolic link.</param>
/// <param name="LinkTarget">Resolved link target when <paramref name="IsSymbolicLink"/> is <see langword="true"/>.</param>
/// <param name="IsTemporary">Whether the file is marked temporary or lives in a temp location.</param>
/// <param name="CreatedAtUtc">Creation time in UTC, when available.</param>
/// <param name="ModifiedAtUtc">Last modification time in UTC, when available.</param>
/// <param name="AccessedAtUtc">Last access time in UTC, when available.</param>
/// <param name="FileType">Content/identity type label, analysis-aware once <see cref="Analysis"/> is populated.</param>
/// <param name="Extension">Name-based extension versus content-detected extension.</param>
/// <param name="Analysis">Optional content-analysis snapshot when requested by the caller.</param>
/// <param name="Icon">Optional OS shell icon pixels when requested by the caller.</param>
public sealed record FileMetadata(
    string Name,
    string DisplayName,
    StorageAttributes Attributes,
    StoragePermissions Permissions,
    bool IsSymbolicLink,
    string? LinkTarget,
    bool IsTemporary,
    DateTimeOffset? CreatedAtUtc,
    DateTimeOffset? ModifiedAtUtc,
    DateTimeOffset? AccessedAtUtc,
    StorageType FileType,
    FileExtension Extension,
    AnalysisReport? Analysis,
    ShellIcon? Icon)
    : StorageMetadata(Name, DisplayName, Attributes, Permissions, IsSymbolicLink, LinkTarget, IsTemporary, CreatedAtUtc, ModifiedAtUtc, AccessedAtUtc);
