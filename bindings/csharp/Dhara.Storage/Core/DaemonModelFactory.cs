using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Models.Watching;
using Dhara.Storage.Sd.V1;

namespace Dhara.Storage.Core;

/// <summary>
/// Builds public model records from <c>dhara-sd</c> gRPC responses.
/// </summary>
/// <remarks>The daemon and this process always run on the same host, so metadata the current
/// protocol does not carry yet (OS attributes, timestamps, link targets) is filled in from
/// <see cref="FileSystemInfo"/> instead of round-tripping for it. Shell icons and shell display
/// details are not available over the daemon transport and are always reported as
/// <see langword="null"/>.</remarks>
internal static class DaemonModelFactory
{
    private static readonly string[] SizeUnits = ["B", "KB", "MB", "GB", "TB", "PB"];

    /// <summary>Builds a <see cref="FileInformation"/> snapshot from a <c>GetFileInfo</c> response.</summary>
    internal static FileInformation ToFileInformation(GetFileInfoResponse response, AnalysisReport? analysis)
    {
        var attributes = TryGetAttributes(response.Path);
        var (created, modified, accessed) = TryGetTimestamps(response.Path);
        var linkTarget = TryGetLinkTarget(response.Path, isDirectory: false);
        var extension = response.HasExtension ? response.Extension : NormalizeExtension(Path.GetExtension(response.Path));

        return new FileInformation(
            response.Path,
            response.Name,
            response.IsReadOnly,
            attributes.HasFlag(FileAttributes.Hidden),
            attributes.HasFlag(FileAttributes.System),
            attributes.HasFlag(FileAttributes.Temporary),
            linkTarget is not null,
            linkTarget,
            created,
            modified,
            accessed,
            response.Name,
            response.Size,
            FormatSize(response.Size),
            extension,
            analysis,
            Icon: null,
            ShellDetails: null);
    }

    /// <summary>Builds a <see cref="DirectoryInformation"/> snapshot from a <c>GetDirectoryInfo</c> response.</summary>
    internal static DirectoryInformation ToDirectoryInformation(GetDirectoryInfoResponse response, DirectorySummary? summary)
    {
        var attributes = TryGetAttributes(response.Path);
        var (created, modified, accessed) = TryGetTimestamps(response.Path);
        var linkTarget = TryGetLinkTarget(response.Path, isDirectory: true);

        return new DirectoryInformation(
            response.Path,
            response.Name,
            attributes.HasFlag(FileAttributes.ReadOnly),
            attributes.HasFlag(FileAttributes.Hidden),
            attributes.HasFlag(FileAttributes.System),
            attributes.HasFlag(FileAttributes.Temporary),
            linkTarget is not null,
            linkTarget,
            created,
            modified,
            accessed,
            response.Name,
            summary,
            Icon: null,
            ShellDetails: null);
    }

    /// <summary>Computes a recursive size/count summary for <paramref name="path"/> from the local filesystem.</summary>
    /// <remarks>There is no daemon RPC for this yet; since the daemon and this process share a
    /// filesystem, walking the tree locally avoids a chatty per-entry round trip.</remarks>
    internal static DirectorySummary BuildDirectorySummary(string path)
    {
        ulong totalSize = 0;
        ulong fileCount = 0;
        ulong directoryCount = 0;

        if (Directory.Exists(path))
        {
            foreach (var entry in EnumerateFileSystemInfosSafely(path))
            {
                if (entry is FileInfo file)
                {
                    fileCount++;
                    totalSize += (ulong)file.Length;
                }
                else if (entry is DirectoryInfo)
                {
                    directoryCount++;
                }
            }
        }

        return new DirectorySummary(totalSize, fileCount, directoryCount, FormatSize(totalSize));
    }

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

    /// <summary>Converts a streamed <c>CopyFileProgress</c> update into the public <see cref="StorageProgress"/> model.</summary>
    internal static StorageProgress ToProgress(CopyFileProgress progress) =>
        new(progress.HasTotalBytes ? progress.TotalBytes : null, progress.BytesTransferred, progress.BytesPerSecond);

    /// <summary>Formats a byte count using the same binary (1024-based) unit ladder as the native runtime.</summary>
    internal static string FormatSize(ulong bytes)
    {
        double size = bytes;
        var unitIndex = 0;
        while (size >= 1024 && unitIndex < SizeUnits.Length - 1)
        {
            size /= 1024;
            unitIndex++;
        }

        return unitIndex == 0 ? $"{bytes} {SizeUnits[0]}" : $"{size:0.##} {SizeUnits[unitIndex]}";
    }

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

    private static FileAttributes TryGetAttributes(string path)
    {
        try
        {
            return File.GetAttributes(path);
        }
        catch (IOException)
        {
            return FileAttributes.Normal;
        }
        catch (UnauthorizedAccessException)
        {
            return FileAttributes.Normal;
        }
    }

    private static (DateTimeOffset? Created, DateTimeOffset? Modified, DateTimeOffset? Accessed) TryGetTimestamps(string path)
    {
        try
        {
            return (
                new DateTimeOffset(File.GetCreationTimeUtc(path), TimeSpan.Zero),
                new DateTimeOffset(File.GetLastWriteTimeUtc(path), TimeSpan.Zero),
                new DateTimeOffset(File.GetLastAccessTimeUtc(path), TimeSpan.Zero));
        }
        catch (IOException)
        {
            return (null, null, null);
        }
        catch (UnauthorizedAccessException)
        {
            return (null, null, null);
        }
    }

    private static string? TryGetLinkTarget(string path, bool isDirectory)
    {
        try
        {
            FileSystemInfo info = isDirectory ? new DirectoryInfo(path) : new FileInfo(path);
            return info.LinkTarget;
        }
        catch (IOException)
        {
            return null;
        }
        catch (UnauthorizedAccessException)
        {
            return null;
        }
    }

    private static IEnumerable<FileSystemInfo> EnumerateFileSystemInfosSafely(string path)
    {
        IEnumerator<FileSystemInfo>? enumerator = null;
        try
        {
            enumerator = new DirectoryInfo(path).EnumerateFileSystemInfos("*", SearchOption.AllDirectories).GetEnumerator();
        }
        catch (IOException)
        {
        }
        catch (UnauthorizedAccessException)
        {
        }

        if (enumerator is null)
        {
            yield break;
        }

        using (enumerator)
        {
            while (true)
            {
                FileSystemInfo current;
                try
                {
                    if (!enumerator.MoveNext())
                    {
                        break;
                    }

                    current = enumerator.Current;
                }
                catch (UnauthorizedAccessException)
                {
                    break;
                }

                yield return current;
            }
        }
    }
}
