namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents common immutable metadata for a storage path.
/// </summary>
public abstract record StorageInformation(
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
    DateTimeOffset? AccessedAtUtc);
