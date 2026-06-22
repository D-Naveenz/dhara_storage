namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents directory-specific metadata returned from the native runtime.
/// </summary>
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
    DirectorySummary? Summary)
    : StorageInformation(Path, Name, IsReadOnly, IsHidden, IsSystem, IsTemporary, IsSymbolicLink, LinkTarget, CreatedAtUtc, ModifiedAtUtc, AccessedAtUtc);
