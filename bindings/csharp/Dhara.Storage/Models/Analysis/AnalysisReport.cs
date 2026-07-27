namespace Dhara.Storage.Models.Analysis;

/// <summary>
/// Represents the native content-analysis result for a file path.
/// </summary>
/// <param name="Matches">Ranked definition matches from signature analysis (strongest first).</param>
/// <param name="TopMimeType">Best-effort MIME type from the top match, when available.</param>
/// <param name="TopDetectedExtension">Best-effort extension candidate from the top match, when available.</param>
/// <param name="ContentKind">High-level classification such as text or binary.</param>
/// <param name="BytesScanned">Number of bytes examined during analysis.</param>
/// <param name="FileSize">Total file size in bytes.</param>
/// <param name="SourceExtension">Extension taken from the source path or name, when present.</param>
public sealed record AnalysisReport(
    IReadOnlyList<DetectedDefinition> Matches,
    string? TopMimeType,
    string? TopDetectedExtension,
    string ContentKind,
    int BytesScanned,
    ulong FileSize,
    string? SourceExtension);
