using Dhara.Storage.Models.Analysis;

namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents file-specific metadata returned from the native runtime.
/// </summary>
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
    AnalysisReport? Analysis)
    : StorageInformation(Path, Name, IsReadOnly, IsHidden, IsSystem, IsTemporary, IsSymbolicLink, LinkTarget, CreatedAtUtc, ModifiedAtUtc, AccessedAtUtc);
