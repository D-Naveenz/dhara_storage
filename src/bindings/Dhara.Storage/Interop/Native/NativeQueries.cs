using System.Runtime.InteropServices;
using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;

namespace Dhara.Storage.Interop.Native;

internal static partial class NativeQueries
{
    private const string LibraryName = NativeMemory.LibraryName;

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_analyze_path(string path, out nint report, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_get_file_info(string path, byte includeAnalysis, out nint info, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_get_directory_info(string path, byte includeSummary, out nint info, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_list_files(string path, byte recursive, out nint entries, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_list_directories(string path, byte recursive, out nint entries, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_list_entries(string path, byte recursive, out nint entries, out nint errorPtr, out nuint errorLen);

    [Obsolete("Legacy JSON ABI. Use typed dhara_analyze_path instead.")]
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_analyze_path_json_old(string path, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [Obsolete("Legacy JSON ABI. Use typed dhara_get_file_info instead.")]
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_get_file_info_json_old(string path, byte includeAnalysis, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [Obsolete("Legacy JSON ABI. Use typed dhara_get_directory_info instead.")]
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_get_directory_info_json_old(string path, byte includeSummary, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [Obsolete("Legacy JSON ABI. Use typed dhara_list_files instead.")]
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_list_files_json_old(string path, byte recursive, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [Obsolete("Legacy JSON ABI. Use typed dhara_list_directories instead.")]
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_list_directories_json_old(string path, byte recursive, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [Obsolete("Legacy JSON ABI. Use typed dhara_list_entries instead.")]
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_list_entries_json_old(string path, byte recursive, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName)]
    internal static partial void dhara_analysis_report_free(nint report);

    [LibraryImport(LibraryName)]
    internal static partial void dhara_file_info_free(nint info);

    [LibraryImport(LibraryName)]
    internal static partial void dhara_directory_info_free(nint info);

    [LibraryImport(LibraryName)]
    internal static partial void dhara_storage_entry_list_free(nint entries);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_read_file(string path, out nint bytesPtr, out nuint bytesLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_read_file_text(string path, out nint stringPtr, out nuint stringLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static unsafe partial NativeStatus dhara_write_file(string path, byte* dataPtr, nuint dataLen, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_write_file_text(string path, string text, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_copy_file(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_move_file(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_rename_file(string source, string newName, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_delete_file(string path, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_create_directory(string path, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_create_directory_all(string path, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_copy_directory(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_move_directory(string source, string destination, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_rename_directory(string source, string newName, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_delete_directory(string path, byte recursive, out nint errorPtr, out nuint errorLen);
}

internal static class NativeQueryInvoker
{
    internal static AnalysisReport AnalyzePath(string path)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeQueries.dhara_analyze_path(path, out var report, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        try
        {
            return NativeTyped.ToAnalysisReport(report);
        }
        finally
        {
            NativeQueries.dhara_analysis_report_free(report);
        }
    }

    internal static FileInformation GetFileInformation(string path, bool includeAnalysis)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeQueries.dhara_get_file_info(path, NativeHelpers.ToNativeBool(includeAnalysis), out var info, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        try
        {
            return NativeTyped.ToFileInformation(info);
        }
        finally
        {
            NativeQueries.dhara_file_info_free(info);
        }
    }

    internal static DirectoryInformation GetDirectoryInformation(string path, bool includeSummary)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeQueries.dhara_get_directory_info(path, NativeHelpers.ToNativeBool(includeSummary), out var info, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        try
        {
            return NativeTyped.ToDirectoryInformation(info);
        }
        finally
        {
            NativeQueries.dhara_directory_info_free(info);
        }
    }

    internal static IReadOnlyList<StorageEntry> ListFiles(string path, bool recursive) =>
        InvokeEntryList((out nint entries, out nint errorPtr, out nuint errorLen) =>
            NativeQueries.dhara_list_files(path, NativeHelpers.ToNativeBool(recursive), out entries, out errorPtr, out errorLen));

    internal static IReadOnlyList<StorageEntry> ListDirectories(string path, bool recursive) =>
        InvokeEntryList((out nint entries, out nint errorPtr, out nuint errorLen) =>
            NativeQueries.dhara_list_directories(path, NativeHelpers.ToNativeBool(recursive), out entries, out errorPtr, out errorLen));

    internal static IReadOnlyList<StorageEntry> ListEntries(string path, bool recursive) =>
        InvokeEntryList((out nint entries, out nint errorPtr, out nuint errorLen) =>
            NativeQueries.dhara_list_entries(path, NativeHelpers.ToNativeBool(recursive), out entries, out errorPtr, out errorLen));

    internal static byte[] ReadFileBytes(string path) =>
        NativeCallInvoker.InvokeBytes(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_read_file(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string ReadFileText(string path) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_read_file_text(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static unsafe string WriteFileBytes(string path, byte[] content)
    {
        fixed (byte* ptr = content)
        {
            var status = NativeQueries.dhara_write_file(path, ptr, (nuint)content.Length, out var dataPtr, out var dataLen, out var errorPtr, out var errorLen);
            NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
            return NativeMemory.ReadUtf8AndFree(dataPtr, dataLen);
        }
    }

    internal static string WriteFileText(string path, string text) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_write_file_text(path, text, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string CopyFile(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_copy_file(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string MoveFile(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_move_file(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string RenameFile(string source, string newName) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_rename_file(source, newName, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static void DeleteFile(string path) =>
        NativeCallInvoker.InvokeUnit((out nint errorPtr, out nuint errorLen) => NativeQueries.dhara_delete_file(path, out errorPtr, out errorLen));

    internal static string CreateDirectory(string path) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_create_directory(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string CreateDirectoryAll(string path) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_create_directory_all(path, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string CopyDirectory(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_copy_directory(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string MoveDirectory(string source, string destination) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_move_directory(source, destination, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static string RenameDirectory(string source, string newName) =>
        NativeCallInvoker.InvokeString(
            (out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen) =>
                NativeQueries.dhara_rename_directory(source, newName, out dataPtr, out dataLen, out errorPtr, out errorLen));

    internal static void DeleteDirectory(string path, bool recursive) =>
        NativeCallInvoker.InvokeUnit((out nint errorPtr, out nuint errorLen) => NativeQueries.dhara_delete_directory(path, NativeHelpers.ToNativeBool(recursive), out errorPtr, out errorLen));

    private delegate NativeStatus NativeEntryListCall(out nint entries, out nint errorPtr, out nuint errorLen);

    private static IReadOnlyList<StorageEntry> InvokeEntryList(NativeEntryListCall call)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = call(out var entries, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        try
        {
            return NativeTyped.ToStorageEntries(entries);
        }
        finally
        {
            NativeQueries.dhara_storage_entry_list_free(entries);
        }
    }
}
