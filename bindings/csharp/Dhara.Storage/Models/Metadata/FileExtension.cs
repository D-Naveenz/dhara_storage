namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents a file's name-based extension versus its content-detected extension.
/// </summary>
/// <param name="Source">Extension from the file name (no leading dot), when present.</param>
/// <param name="Detected">Extension detected by content analysis, when analysis has run.</param>
/// <param name="Display">Formatted label such as <c>"JPEG"</c>, or <c>"JPEG (PNG)"</c> when source and detected differ.</param>
public sealed record FileExtension(string? Source, string? Detected, string Display);
