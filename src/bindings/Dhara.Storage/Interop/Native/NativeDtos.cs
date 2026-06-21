namespace Dhara.Storage.Interop.Native;

internal sealed record NativeErrorPayload(
    string Code,
    string Message,
    string? Path,
    string? Operation,
    string? Kind,
    string? Value);

internal sealed record NativeStorageMetadataDto(
    string Path,
    string Name,
    bool IsReadOnly,
    bool IsHidden,
    bool IsSystem,
    bool IsTemporary,
    bool IsSymbolicLink,
    string? LinkTarget,
    long? CreatedAtUtcMs,
    long? ModifiedAtUtcMs,
    long? AccessedAtUtcMs);

internal sealed record NativeDetectedDefinitionDto(
    string FileTypeLabel,
    string MimeType,
    string[] Extensions,
    ulong Score,
    double Confidence);

internal sealed record NativeAnalysisReportDto(
    NativeDetectedDefinitionDto[] Matches,
    string? TopMimeType,
    string? TopDetectedExtension,
    string ContentKind,
    int BytesScanned,
    ulong FileSize,
    string? SourceExtension);

internal sealed record NativeFileInformationDto(
    NativeStorageMetadataDto Metadata,
    string DisplayName,
    ulong Size,
    string FormattedSize,
    string? FilenameExtension,
    NativeAnalysisReportDto? Analysis);

internal sealed record NativeDirectorySummaryDto(
    ulong TotalSize,
    ulong FileCount,
    ulong DirectoryCount,
    string FormattedSize);

internal sealed record NativeDirectoryInformationDto(
    NativeStorageMetadataDto Metadata,
    string DisplayName,
    NativeDirectorySummaryDto? Summary);

internal sealed record NativeStorageEntryDto(
    string Kind,
    string Path,
    string Name);

internal sealed record NativeWatchEventDto(
    string ChangeType,
    string Path,
    string? PreviousPath,
    long ObservedAtUtcMs);

internal sealed record NativeLogRecordDto(
    string Level,
    string Target,
    string Message,
    ulong TimestampUnixMs,
    string? ModulePath,
    string? File,
    uint? Line,
    Dictionary<string, string> Fields);
