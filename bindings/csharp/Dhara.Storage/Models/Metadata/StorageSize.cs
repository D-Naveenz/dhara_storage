namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents an on-demand size measurement for a storage path.
/// </summary>
/// <param name="Bytes">Size in bytes as reported by the operating system (or directory walk total).</param>
/// <param name="Formatted">Human-readable form using binary units (for example <c>"2.36 KiB"</c>).</param>
/// <remarks>Paths and size live on <see cref="StorageFile"/> / <see cref="StorageDirectory"/>, not on
/// <see cref="StorageMetadata"/>; size is measured when requested and is not cached as part of a
/// metadata snapshot.</remarks>
public sealed record StorageSize(ulong Bytes, string Formatted);
