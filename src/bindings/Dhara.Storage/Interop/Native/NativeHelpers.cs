using Dhara.Storage.Exceptions;
using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Models.Watching;
using System.Runtime.InteropServices;
using Microsoft.Extensions.Logging;

namespace Dhara.Storage.Interop.Native;

internal static class NativeHelpers
{
    internal static byte ToNativeBool(bool value) => value ? (byte)1 : (byte)0;

    internal static void EnsureSupportedPlatform()
    {
        if (OperatingSystem.IsWindows() &&
            (RuntimeInformation.ProcessArchitecture == Architecture.X64 ||
             RuntimeInformation.ProcessArchitecture == Architecture.Arm64))
        {
            return;
        }

        throw new PlatformNotSupportedException(
            $"Dhara.Storage supports Windows x64 and Windows arm64 only. Current platform: {RuntimeInformation.OSDescription}, architecture: {RuntimeInformation.ProcessArchitecture}.");
    }

    internal static void ThrowIfFailed(NativeStatus status, nint errorPtr, nuint errorLen)
    {
        if (status == NativeStatus.Ok)
        {
            return;
        }

        var payloadJson = errorPtr == 0 ? null : NativeMemory.ReadUtf8AndFree(errorPtr, errorLen);
        if (string.IsNullOrWhiteSpace(payloadJson))
        {
            throw new DharaStorageException($"Native call failed with status {status}.", status.ToString());
        }

        var payload = NativeJson.Deserialize<NativeErrorPayload>(payloadJson);
        if (string.Equals(payload.Code, "cancelled", StringComparison.OrdinalIgnoreCase))
        {
            throw new OperationCanceledException(payload.Message);
        }

        throw new DharaStorageException(payload.Message, payload.Code, payload.Path, payload.Operation);
    }

    internal static void ThrowIfFailed(NativeStatus status, nint errorPtr, nuint errorLen, CancellationToken cancellationToken)
    {
        try
        {
            ThrowIfFailed(status, errorPtr, errorLen);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
    }

    internal static AnalysisReport ToModel(this NativeAnalysisReportDto dto) =>
        new(
            dto.Matches.Select(ToModel).ToArray(),
            dto.TopMimeType,
            dto.TopDetectedExtension,
            dto.ContentKind,
            dto.BytesScanned,
            dto.FileSize,
            dto.SourceExtension);

    internal static DetectedDefinition ToModel(this NativeDetectedDefinitionDto dto) =>
        new(dto.FileTypeLabel, dto.MimeType, dto.Extensions, dto.Score, dto.Confidence);

    internal static FileInformation ToModel(this NativeFileInformationDto dto)
    {
        var metadata = dto.Metadata;
        return new FileInformation(
            metadata.Path,
            metadata.Name,
            metadata.IsReadOnly,
            metadata.IsHidden,
            metadata.IsSystem,
            metadata.IsTemporary,
            metadata.IsSymbolicLink,
            metadata.LinkTarget,
            ToDateTimeOffset(metadata.CreatedAtUtcMs),
            ToDateTimeOffset(metadata.ModifiedAtUtcMs),
            ToDateTimeOffset(metadata.AccessedAtUtcMs),
            dto.DisplayName,
            dto.Size,
            dto.FormattedSize,
            dto.FilenameExtension,
            dto.Analysis?.ToModel());
    }

    internal static DirectoryInformation ToModel(this NativeDirectoryInformationDto dto)
    {
        var metadata = dto.Metadata;
        return new DirectoryInformation(
            metadata.Path,
            metadata.Name,
            metadata.IsReadOnly,
            metadata.IsHidden,
            metadata.IsSystem,
            metadata.IsTemporary,
            metadata.IsSymbolicLink,
            metadata.LinkTarget,
            ToDateTimeOffset(metadata.CreatedAtUtcMs),
            ToDateTimeOffset(metadata.ModifiedAtUtcMs),
            ToDateTimeOffset(metadata.AccessedAtUtcMs),
            dto.DisplayName,
            dto.Summary is null
                ? null
                : new DirectorySummary(dto.Summary.TotalSize, dto.Summary.FileCount, dto.Summary.DirectoryCount, dto.Summary.FormattedSize));
    }

    internal static StorageEntry ToModel(this NativeStorageEntryDto dto) => new(dto.Kind, dto.Path, dto.Name);

    internal static StorageChangedEventArgs ToModel(this NativeWatchEventDto dto) =>
        new(
            dto.Path,
            dto.PreviousPath,
            dto.ChangeType switch
            {
                "created" => StorageChangeType.Created,
                "deleted" => StorageChangeType.Deleted,
                "modified" => StorageChangeType.Modified,
                "relocated" => StorageChangeType.Relocated,
                _ => StorageChangeType.Modified,
            },
            DateTimeOffset.FromUnixTimeMilliseconds(dto.ObservedAtUtcMs));

    internal static StorageProgress ToModel(this NativeOperationSnapshot snapshot) =>
        new(
            snapshot.HasTotalBytes == 0 ? null : snapshot.TotalBytes,
            snapshot.BytesTransferred,
            snapshot.BytesPerSecond);

    internal static LogLevel ToLogLevel(this NativeLogRecordDto record) =>
        record.Level.ToLogLevel();

    internal static LogLevel ToLogLevel(this string level) =>
        level.ToUpperInvariant() switch
        {
            "TRACE" => LogLevel.Trace,
            "DEBUG" => LogLevel.Debug,
            "INFO" => LogLevel.Information,
            "WARN" => LogLevel.Warning,
            "ERROR" => LogLevel.Error,
            _ => LogLevel.Information,
        };

    private static DateTimeOffset? ToDateTimeOffset(long? unixMilliseconds) =>
        unixMilliseconds.HasValue ? DateTimeOffset.FromUnixTimeMilliseconds(unixMilliseconds.Value) : null;
}
