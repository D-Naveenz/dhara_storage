namespace Dhara.Storage.Models.Analysis;

/// <summary>
/// Represents the native content-analysis result for a file path.
/// </summary>
public sealed record AnalysisReport(
    IReadOnlyList<DetectedDefinition> Matches,
    string? TopMimeType,
    string? TopDetectedExtension,
    string ContentKind,
    int BytesScanned,
    ulong FileSize,
    string? SourceExtension);
