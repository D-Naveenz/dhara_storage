namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents a human type label plus an optional MIME type for a file.
/// </summary>
/// <param name="Name">Human-friendly type label (for example <c>"PNG Image"</c> or <c>"JSON Source File"</c>).</param>
/// <param name="MimeType">MIME type when known (for example <c>"image/png"</c>), typically populated once content analysis has run.</param>
public sealed record StorageType(string Name, string? MimeType);
