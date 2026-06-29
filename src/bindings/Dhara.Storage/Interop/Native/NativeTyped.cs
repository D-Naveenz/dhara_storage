using System.Runtime.InteropServices;
using System.Text;
using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;
using Dhara.Storage.Models.Watching;

namespace Dhara.Storage.Interop.Native;

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeUtf8
{
    internal readonly nint Ptr;
    internal readonly nuint Len;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeOptionalUtf8
{
    internal readonly byte HasValue;
    internal readonly NativeUtf8 Value;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeStorageMetadataV2
{
    internal readonly NativeUtf8 Path;
    internal readonly NativeUtf8 Name;
    internal readonly byte IsReadOnly;
    internal readonly byte IsHidden;
    internal readonly byte IsSystem;
    internal readonly byte IsTemporary;
    internal readonly byte IsSymbolicLink;
    internal readonly NativeOptionalUtf8 LinkTarget;
    internal readonly byte HasCreatedAtUtcMs;
    internal readonly ulong CreatedAtUtcMs;
    internal readonly byte HasModifiedAtUtcMs;
    internal readonly ulong ModifiedAtUtcMs;
    internal readonly byte HasAccessedAtUtcMs;
    internal readonly ulong AccessedAtUtcMs;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeDirectorySummaryV2
{
    internal readonly ulong TotalSize;
    internal readonly ulong FileCount;
    internal readonly ulong DirectoryCount;
    internal readonly NativeUtf8 FormattedSize;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeDetectedDefinitionV2
{
    internal readonly NativeUtf8 FileTypeLabel;
    internal readonly NativeUtf8 MimeType;
    internal readonly nint ExtensionsPtr;
    internal readonly nuint ExtensionsLen;
    internal readonly ulong Score;
    internal readonly double Confidence;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeAnalysisReportV2
{
    internal readonly nint MatchesPtr;
    internal readonly nuint MatchesLen;
    internal readonly NativeOptionalUtf8 TopMimeType;
    internal readonly NativeOptionalUtf8 TopDetectedExtension;
    internal readonly uint ContentKind;
    internal readonly nuint BytesScanned;
    internal readonly ulong FileSize;
    internal readonly NativeOptionalUtf8 SourceExtension;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeShellIconV2
{
    internal readonly uint Width;
    internal readonly uint Height;
    internal readonly nint PixelsPtr;
    internal readonly nuint PixelsLen;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeShellDetailsV2
{
    internal readonly byte HasValue;
    internal readonly NativeOptionalUtf8 DisplayName;
    internal readonly NativeOptionalUtf8 TypeName;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeFileInformationV2
{
    internal readonly NativeStorageMetadataV2 Metadata;
    internal readonly NativeUtf8 DisplayName;
    internal readonly ulong Size;
    internal readonly NativeUtf8 FormattedSize;
    internal readonly NativeOptionalUtf8 FilenameExtension;
    internal readonly nint Analysis;
    internal readonly byte HasIcon;
    internal readonly NativeShellIconV2 Icon;
    internal readonly NativeShellDetailsV2 ShellDetails;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeDirectoryInformationV2
{
    internal readonly NativeStorageMetadataV2 Metadata;
    internal readonly NativeUtf8 DisplayName;
    internal readonly byte HasSummary;
    internal readonly NativeDirectorySummaryV2 Summary;
    internal readonly byte HasIcon;
    internal readonly NativeShellIconV2 Icon;
    internal readonly NativeShellDetailsV2 ShellDetails;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeStorageEntryV2
{
    internal readonly uint Kind;
    internal readonly NativeUtf8 Path;
    internal readonly NativeUtf8 Name;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeStorageEntryListV2
{
    internal readonly nint EntriesPtr;
    internal readonly nuint EntriesLen;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeWatchEventV2
{
    internal readonly uint ChangeType;
    internal readonly NativeUtf8 Path;
    internal readonly NativeOptionalUtf8 PreviousPath;
    internal readonly ulong ObservedAtUtcMs;
}

internal static unsafe class NativeTyped
{
    internal static AnalysisReport ToAnalysisReport(nint reportPtr)
    {
        var report = Read<NativeAnalysisReportV2>(reportPtr);
        var nativeMatches = ReadSpan<NativeDetectedDefinitionV2>(report.MatchesPtr, report.MatchesLen);
        var matches = new DetectedDefinition[nativeMatches.Length];
        for (var i = 0; i < nativeMatches.Length; i++)
        {
            matches[i] = ToDetectedDefinition(nativeMatches[i]);
        }

        return new AnalysisReport(
            matches,
            ToNullableString(report.TopMimeType),
            ToNullableString(report.TopDetectedExtension),
            ToContentKind(report.ContentKind),
            checked((int)report.BytesScanned),
            report.FileSize,
            ToNullableString(report.SourceExtension));
    }

    internal static FileInformation ToFileInformation(nint infoPtr)
    {
        var info = Read<NativeFileInformationV2>(infoPtr);
        var metadata = info.Metadata;
        return new FileInformation(
            ToString(metadata.Path),
            ToString(metadata.Name),
            metadata.IsReadOnly != 0,
            metadata.IsHidden != 0,
            metadata.IsSystem != 0,
            metadata.IsTemporary != 0,
            metadata.IsSymbolicLink != 0,
            ToNullableString(metadata.LinkTarget),
            ToDateTimeOffset(metadata.HasCreatedAtUtcMs, metadata.CreatedAtUtcMs),
            ToDateTimeOffset(metadata.HasModifiedAtUtcMs, metadata.ModifiedAtUtcMs),
            ToDateTimeOffset(metadata.HasAccessedAtUtcMs, metadata.AccessedAtUtcMs),
            ToString(info.DisplayName),
            info.Size,
            ToString(info.FormattedSize),
            ToNullableString(info.FilenameExtension),
            info.Analysis == 0 ? null : ToAnalysisReport(info.Analysis),
            ToShellIcon(info.HasIcon, info.Icon),
            ToShellDetails(info.ShellDetails));
    }

    internal static DirectoryInformation ToDirectoryInformation(nint infoPtr)
    {
        var info = Read<NativeDirectoryInformationV2>(infoPtr);
        var metadata = info.Metadata;
        return new DirectoryInformation(
            ToString(metadata.Path),
            ToString(metadata.Name),
            metadata.IsReadOnly != 0,
            metadata.IsHidden != 0,
            metadata.IsSystem != 0,
            metadata.IsTemporary != 0,
            metadata.IsSymbolicLink != 0,
            ToNullableString(metadata.LinkTarget),
            ToDateTimeOffset(metadata.HasCreatedAtUtcMs, metadata.CreatedAtUtcMs),
            ToDateTimeOffset(metadata.HasModifiedAtUtcMs, metadata.ModifiedAtUtcMs),
            ToDateTimeOffset(metadata.HasAccessedAtUtcMs, metadata.AccessedAtUtcMs),
            ToString(info.DisplayName),
            info.HasSummary == 0
                ? null
                : new DirectorySummary(
                    info.Summary.TotalSize,
                    info.Summary.FileCount,
                    info.Summary.DirectoryCount,
                    ToString(info.Summary.FormattedSize)),
            ToShellIcon(info.HasIcon, info.Icon),
            ToShellDetails(info.ShellDetails));
    }

    internal static IReadOnlyList<StorageEntry> ToStorageEntries(nint entriesPtr)
    {
        var list = Read<NativeStorageEntryListV2>(entriesPtr);
        var nativeEntries = ReadSpan<NativeStorageEntryV2>(list.EntriesPtr, list.EntriesLen);
        var entries = new StorageEntry[nativeEntries.Length];
        for (var i = 0; i < nativeEntries.Length; i++)
        {
            var entry = nativeEntries[i];
            entries[i] = new StorageEntry(ToEntryKind(entry.Kind), ToString(entry.Path), ToString(entry.Name));
        }

        return entries;
    }

    internal static StorageChangedEventArgs ToWatchEvent(nint eventPtr)
    {
        var nativeEvent = Read<NativeWatchEventV2>(eventPtr);
        return new StorageChangedEventArgs(
            ToString(nativeEvent.Path),
            ToNullableString(nativeEvent.PreviousPath),
            ToStorageChangeType(nativeEvent.ChangeType),
            DateTimeOffset.FromUnixTimeMilliseconds(checked((long)nativeEvent.ObservedAtUtcMs)));
    }

    private static ShellIcon? ToShellIcon(byte hasIcon, NativeShellIconV2 icon)
    {
        if (hasIcon == 0 || icon.PixelsPtr == 0 || icon.PixelsLen == 0)
        {
            return null;
        }

        var pixels = ReadBytes(icon.PixelsPtr, icon.PixelsLen);
        return new ShellIcon(checked((int)icon.Width), checked((int)icon.Height), pixels);
    }

    private static ShellDetails? ToShellDetails(NativeShellDetailsV2 details)
    {
        if (details.HasValue == 0)
        {
            return null;
        }

        return new ShellDetails(
            ToNullableString(details.DisplayName),
            ToNullableString(details.TypeName));
    }

    private static byte[] ReadBytes(nint ptr, nuint len)
    {
        if (len == 0)
        {
            return [];
        }

        if (ptr == 0)
        {
            throw new InvalidOperationException("Native typed payload byte pointer was null.");
        }

        var bytes = new byte[checked((int)len)];
        Marshal.Copy(ptr, bytes, 0, bytes.Length);
        return bytes;
    }

    private static DetectedDefinition ToDetectedDefinition(NativeDetectedDefinitionV2 definition)
    {
        var nativeExtensions = ReadSpan<NativeUtf8>(definition.ExtensionsPtr, definition.ExtensionsLen);
        var extensions = new string[nativeExtensions.Length];
        for (var i = 0; i < nativeExtensions.Length; i++)
        {
            extensions[i] = ToString(nativeExtensions[i]);
        }

        return new DetectedDefinition(
            ToString(definition.FileTypeLabel),
            ToString(definition.MimeType),
            extensions,
            definition.Score,
            definition.Confidence);
    }

    private static string ToContentKind(uint value) =>
        value switch
        {
            0 => "text",
            1 => "binary",
            _ => "unknown",
        };

    private static string ToEntryKind(uint value) => value == 1 ? "directory" : "file";

    private static StorageChangeType ToStorageChangeType(uint value) =>
        value switch
        {
            0 => StorageChangeType.Created,
            1 => StorageChangeType.Deleted,
            3 => StorageChangeType.Relocated,
            _ => StorageChangeType.Modified,
        };

    private static string? ToNullableString(NativeOptionalUtf8 value) =>
        value.HasValue == 0 ? null : ToString(value.Value);

    private static string ToString(NativeUtf8 value)
    {
        if (value.Ptr == 0 || value.Len == 0)
        {
            return string.Empty;
        }

        return Encoding.UTF8.GetString(new ReadOnlySpan<byte>((void*)value.Ptr, checked((int)value.Len)));
    }

    private static DateTimeOffset? ToDateTimeOffset(byte hasValue, ulong value) =>
        hasValue == 0 ? null : DateTimeOffset.FromUnixTimeMilliseconds(checked((long)value));

    private static T Read<T>(nint ptr)
        where T : unmanaged
    {
        if (ptr == 0)
        {
            throw new InvalidOperationException("Native typed payload pointer was null.");
        }

        return *(T*)ptr;
    }

    private static ReadOnlySpan<T> ReadSpan<T>(nint ptr, nuint len)
        where T : unmanaged
    {
        if (len == 0)
        {
            return ReadOnlySpan<T>.Empty;
        }

        if (ptr == 0)
        {
            throw new InvalidOperationException("Native typed payload slice pointer was null.");
        }

        return new ReadOnlySpan<T>((void*)ptr, checked((int)len));
    }
}
