namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents common immutable metadata for a storage path.
/// </summary>
/// <param name="Name">Last path segment (file or directory name).</param>
/// <param name="DisplayName">Shell display name when available; otherwise a name-based fallback.</param>
/// <param name="Attributes">Settable filesystem attributes snapshot.</param>
/// <param name="Permissions">Effective permissions snapshot for the current process.</param>
/// <param name="IsSymbolicLink">Whether the path is a symbolic link.</param>
/// <param name="LinkTarget">Resolved link target when <paramref name="IsSymbolicLink"/> is <see langword="true"/>.</param>
/// <param name="IsTemporary">Whether the item is marked temporary or lives in a temp location.</param>
/// <param name="CreatedAtUtc">Creation time in UTC, when the platform provides it.</param>
/// <param name="ModifiedAtUtc">Last modification time in UTC, when the platform provides it.</param>
/// <param name="AccessedAtUtc">Last access time in UTC, when the platform provides it.</param>
/// <remarks>Paths and size intentionally live on storage handles
/// (<see cref="StorageFile"/> / <see cref="StorageDirectory"/>), not on metadata snapshots.</remarks>
public abstract record StorageMetadata(
    string Name,
    string DisplayName,
    StorageAttributes Attributes,
    StoragePermissions Permissions,
    bool IsSymbolicLink,
    string? LinkTarget,
    bool IsTemporary,
    DateTimeOffset? CreatedAtUtc,
    DateTimeOffset? ModifiedAtUtc,
    DateTimeOffset? AccessedAtUtc);
