namespace Dhara.Storage.Models.Analysis;

/// <summary>
/// Represents a file-definition match returned by native content analysis.
/// </summary>
public sealed record DetectedDefinition(
    string FileTypeLabel,
    string MimeType,
    IReadOnlyList<string> Extensions,
    ulong Score,
    double Confidence);
