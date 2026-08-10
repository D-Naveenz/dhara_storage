using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Metadata;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Models.Watching;
using Dhara.Storage.Sd.V1;

namespace Dhara.Storage.Core;

/// <summary>
/// Builds public model records from <c>dhara-sd</c> gRPC responses.
/// </summary>
/// <remarks><c>GetFileMetadata</c> / <c>GetDirectoryMetadata</c> responses already carry attributes,
/// permissions, timestamps, link targets, and (for directories) recursive summaries computed natively;
/// this factory only maps the wire shapes onto public records, it does not re-derive metadata locally.</remarks>
internal static class DaemonModelFactory
{
    /// <summary>Builds a <see cref="FileMetadata"/> snapshot from a <c>GetFileMetadata</c> response.</summary>
    internal static FileMetadata ToFileMetadata(GetFileMetadataResponse response, AnalysisReport? analysis) =>
        new(
            response.Name,
            response.DisplayName,
            ToAttributes(response.Attributes),
            ToPermissions(response.Permissions),
            response.IsSymbolicLink,
            response.HasLinkTarget ? response.LinkTarget : null,
            response.IsTemporary,
            ToTimestamp(response.HasCreatedAtUnixMs, response.CreatedAtUnixMs),
            ToTimestamp(response.HasModifiedAtUnixMs, response.ModifiedAtUnixMs),
            ToTimestamp(response.HasAccessedAtUnixMs, response.AccessedAtUnixMs),
            ToStorageType(response.FileType),
            ToFileExtension(response.Extension),
            analysis,
            ToShellIcon(response.Icon));

    /// <summary>
    /// Enriches a metadata snapshot from a handle-cached <see cref="AnalysisReport"/>
    /// (type name, MIME, and detected extension), matching native <c>FileMetadata::apply_analysis</c>.
    /// </summary>
    internal static FileMetadata EnrichFileMetadata(FileMetadata metadata, AnalysisReport analysis)
    {
        var topLabel = analysis.Matches.Count > 0 ? analysis.Matches[0].FileTypeLabel : null;
        var typeName = string.IsNullOrEmpty(topLabel) ? metadata.FileType.Name : topLabel;
        var fileType = new StorageType(typeName, analysis.TopMimeType ?? metadata.FileType.MimeType);
        var extension = new FileExtension(
            metadata.Extension.Source,
            analysis.TopDetectedExtension,
            FormatExtensionDisplay(metadata.Extension.Source, analysis.TopDetectedExtension));
        return metadata with
        {
            FileType = fileType,
            Extension = extension,
            Analysis = analysis,
        };
    }

    /// <summary>Builds a <see cref="DirectoryMetadata"/> snapshot from a <c>GetDirectoryMetadata</c> response.</summary>
    internal static DirectoryMetadata ToDirectoryMetadata(GetDirectoryMetadataResponse response) =>
        new(
            response.Name,
            response.DisplayName,
            ToAttributes(response.Attributes),
            ToPermissions(response.Permissions),
            response.IsSymbolicLink,
            response.HasLinkTarget ? response.LinkTarget : null,
            response.IsTemporary,
            ToTimestamp(response.HasCreatedAtUnixMs, response.CreatedAtUnixMs),
            ToTimestamp(response.HasModifiedAtUnixMs, response.ModifiedAtUnixMs),
            ToTimestamp(response.HasAccessedAtUnixMs, response.AccessedAtUnixMs),
            response.TypeName,
            ToDirectorySummary(response),
            ToShellIcon(response.Icon));

    /// <summary>Builds an <see cref="AnalysisReport"/> from an <c>AnalyzePath</c> response.</summary>
    internal static AnalysisReport ToAnalysisReport(AnalyzePathResponse response, string path)
    {
        var matches = response.Matches
            .Select(static match => new DetectedDefinition(
                match.FileTypeLabel,
                match.MimeType,
                Array.Empty<string>(),
                match.Score,
                match.Confidence))
            .ToArray();

        return new AnalysisReport(
            matches,
            response.HasTopMimeType ? response.TopMimeType : null,
            response.HasTopDetectedExtension ? response.TopDetectedExtension : null,
            ContentKindLabel(response.ContentKind),
            checked((int)response.BytesScanned),
            response.FileSize,
            NormalizeExtension(Path.GetExtension(path)));
    }

    /// <summary>Converts a <c>ListEntries</c> entry into the public <see cref="StorageEntry"/> model.</summary>
    internal static StorageEntry ToStorageEntry(EntryInfo entry) =>
        new(entry.IsDirectory ? "directory" : "file", entry.Path, entry.Name);

    /// <summary>Converts a streamed <c>WatchEvent</c> into the public <see cref="StorageChangedEventArgs"/> model.</summary>
    internal static StorageChangedEventArgs ToChangedEventArgs(WatchEvent watchEvent) =>
        new(
            watchEvent.Path,
            watchEvent.HasPreviousPath ? watchEvent.PreviousPath : null,
            ToChangeType(watchEvent.ChangeType),
            DateTimeOffset.FromUnixTimeMilliseconds(watchEvent.ObservedUnixMillis));

    /// <summary>Converts a streamed <c>StorageProcessEventMessage</c> into the public process event model.</summary>
    internal static StorageProcessEvent? ToProcessEvent(StorageProcessEventMessage message) =>
        message.KindCase switch
        {
            StorageProcessEventMessage.KindOneofCase.Started =>
                new StorageProcessStarted(message.Started.TotalBytes, message.Started.TotalFiles),
            StorageProcessEventMessage.KindOneofCase.CurrentItem =>
                new StorageProcessCurrentItem(
                    message.CurrentItem.Path,
                    message.CurrentItem.FileSize,
                    message.CurrentItem.FileIndex),
            StorageProcessEventMessage.KindOneofCase.Bytes =>
                new StorageProcessBytes(message.Bytes.BytesTransferred),
            StorageProcessEventMessage.KindOneofCase.Completed =>
                new StorageProcessCompleted(message.Completed.HasDestination ? message.Completed.Destination : null),
            StorageProcessEventMessage.KindOneofCase.Failed =>
                new StorageProcessFailed(message.Failed.Message),
            StorageProcessEventMessage.KindOneofCase.Cancelled =>
                new StorageProcessCancelled(message.Cancelled.Operation),
            _ => null,
        };

    /// <summary>Maps a wire size payload onto the public <see cref="StorageSize"/> model.</summary>
    /// <remarks>Byte counts and formatted labels are computed natively; this factory does not
    /// re-derive formatting locally.</remarks>
    internal static StorageSize ToStorageSize(StorageSizePayload payload) =>
        new(payload.Bytes, payload.Formatted);

    private static StorageAttributes ToAttributes(StorageAttributesPayload? payload) =>
        payload is null
            ? new StorageAttributes(false, false, false, false)
            : new StorageAttributes(payload.ReadOnly, payload.Hidden, payload.System, payload.Archive);

    private static StoragePermissions ToPermissions(StoragePermissionsPayload? payload) =>
        payload is null
            ? new StoragePermissions(false, false, false, false)
            : new StoragePermissions(payload.CanRead, payload.CanWrite, payload.CanModify, payload.CanExecute);

    private static StorageType ToStorageType(StorageTypePayload? payload) =>
        payload is null
            ? new StorageType(string.Empty, null)
            : new StorageType(payload.Name, payload.HasMimeType ? payload.MimeType : null);

    private static FileExtension ToFileExtension(FileExtensionPayload? payload) =>
        payload is null
            ? new FileExtension(null, null, string.Empty)
            : new FileExtension(
                payload.HasSource ? payload.Source : null,
                payload.HasDetected ? payload.Detected : null,
                payload.Display);

    private static DirectorySummary? ToDirectorySummary(GetDirectoryMetadataResponse response)
    {
        if (response.Size is null)
        {
            return null;
        }

        return new DirectorySummary(
            new StorageSize(response.Size.Bytes, response.Size.Formatted),
            response.HasFileCount ? response.FileCount : 0,
            response.HasDirectoryCount ? response.DirectoryCount : 0);
    }

    private static ShellIcon? ToShellIcon(ShellIconPayload? payload)
    {
        if (payload is null || (payload.Width == 0 && payload.Height == 0 && payload.RgbaPixels.Length == 0))
        {
            return null;
        }

        return new ShellIcon(
            checked((int)payload.Width),
            checked((int)payload.Height),
            payload.RgbaPixels.Memory);
    }

    private static DateTimeOffset? ToTimestamp(bool hasValue, long unixMillis) =>
        hasValue ? DateTimeOffset.FromUnixTimeMilliseconds(unixMillis) : null;

    private static StorageChangeType ToChangeType(uint value) => value switch
    {
        1 => StorageChangeType.Created,
        2 => StorageChangeType.Deleted,
        3 => StorageChangeType.Modified,
        4 => StorageChangeType.Relocated,
        _ => StorageChangeType.Modified,
    };

    private static string ContentKindLabel(uint kind) => kind switch
    {
        1 => "text",
        2 => "binary",
        _ => "unknown",
    };

    private static string? NormalizeExtension(string? extension) =>
        string.IsNullOrEmpty(extension) ? null : extension.TrimStart('.');

    private static string FormatExtensionDisplay(string? source, string? detected)
    {
        static string? Upper(string? value) =>
            string.IsNullOrEmpty(value) ? null : value.ToUpperInvariant();

        var src = Upper(source);
        var det = Upper(detected);
        return (src, det) switch
        {
            (null, null) => string.Empty,
            (not null, null) => src,
            (null, not null) => det,
            (not null, not null) when src == det => src,
            _ => $"{src} ({det})",
        };
    }
}
