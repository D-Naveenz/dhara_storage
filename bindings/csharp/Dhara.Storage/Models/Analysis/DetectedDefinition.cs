namespace Dhara.Storage.Models.Analysis;

/// <summary>
/// Represents a file-definition match returned by native content analysis.
/// </summary>
/// <param name="FileTypeLabel">Human-readable type label from the definition database.</param>
/// <param name="MimeType">MIME type associated with the match, when known.</param>
/// <param name="Extensions">Extension candidates associated with the match.</param>
/// <param name="Score">Raw ranking score from the analyzer (higher is stronger).</param>
/// <param name="Confidence">Normalized confidence in the range reported by the native runtime.</param>
public sealed record DetectedDefinition(
    string FileTypeLabel,
    string MimeType,
    IReadOnlyList<string> Extensions,
    ulong Score,
    double Confidence);
