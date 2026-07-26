namespace Dhara.Storage.Models.Information;

/// <summary>
/// Represents common immutable metadata for a storage path.
/// </summary>
/// <param name="Path">Absolute or normalized path returned by the native runtime.</param>
/// <param name="Name">Leaf name of the item.</param>
/// <param name="IsReadOnly">Whether the item is marked read-only.</param>
/// <param name="IsHidden">Whether the item is marked hidden.</param>
/// <param name="IsSystem">Whether the item is marked as a system entry.</param>
/// <param name="IsTemporary">Whether the item is marked temporary.</param>
/// <param name="IsSymbolicLink">Whether the path is a symbolic link.</param>
/// <param name="LinkTarget">Resolved link target when <paramref name="IsSymbolicLink"/> is <see langword="true"/>.</param>
/// <param name="CreatedAtUtc">Creation time in UTC, when the platform provides it.</param>
/// <param name="ModifiedAtUtc">Last modification time in UTC, when the platform provides it.</param>
/// <param name="AccessedAtUtc">Last access time in UTC, when the platform provides it.</param>
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
