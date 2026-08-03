using System.Runtime.InteropServices;

namespace Dhara.Storage.Benchmarks;

/// <summary>
/// Bench-only P/Invoke into <c>dharastorage</c> for rung-1 FFI evidence (not used by the NuGet).
/// </summary>
internal static partial class FfiBaseline
{
    private const string LibraryName = "dharastorage";

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    private static partial int dhara_get_file_info(
        string path,
        byte includeAnalysis,
        byte includeIcon,
        uint iconSize,
        out nint outInfo,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName)]
    private static partial void dhara_file_info_free(nint info);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    private static partial int dhara_analyze_path(
        string path,
        out nint outReport,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName)]
    private static partial void dhara_analysis_report_free(nint report);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    private static partial int dhara_list_entries(
        string path,
        byte recursive,
        out nint outList,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName)]
    private static partial void dhara_storage_entry_list_free(nint list);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    private static partial int dhara_read_file(
        string path,
        out nint outBytes,
        out nuint outLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName)]
    private static partial void dhara_bytes_free(nint ptr, nuint len);

    [LibraryImport(LibraryName)]
    private static partial void dhara_string_free(nint ptr, nuint len);

    public static void GetFileInfo(string path)
    {
        var code = dhara_get_file_info(path, 0, 0, 0, out var info, out var errPtr, out var errLen);
        FreeError(errPtr, errLen);
        try
        {
            if (code != 0 || info == 0)
            {
                throw new InvalidOperationException($"dhara_get_file_info failed: {code}");
            }
        }
        finally
        {
            if (info != 0)
            {
                dhara_file_info_free(info);
            }
        }
    }

    public static void AnalyzePath(string path)
    {
        var code = dhara_analyze_path(path, out var report, out var errPtr, out var errLen);
        FreeError(errPtr, errLen);
        try
        {
            if (code != 0 || report == 0)
            {
                throw new InvalidOperationException($"dhara_analyze_path failed: {code}");
            }
        }
        finally
        {
            if (report != 0)
            {
                dhara_analysis_report_free(report);
            }
        }
    }

    public static void ListEntries(string path)
    {
        var code = dhara_list_entries(path, 0, out var list, out var errPtr, out var errLen);
        FreeError(errPtr, errLen);
        try
        {
            if (code != 0 || list == 0)
            {
                throw new InvalidOperationException($"dhara_list_entries failed: {code}");
            }
        }
        finally
        {
            if (list != 0)
            {
                dhara_storage_entry_list_free(list);
            }
        }
    }

    public static void ReadBytes(string path)
    {
        var code = dhara_read_file(path, out var buffer, out var length, out var errPtr, out var errLen);
        FreeError(errPtr, errLen);
        try
        {
            if (code != 0 || buffer == 0)
            {
                throw new InvalidOperationException($"dhara_read_file failed: {code}");
            }
        }
        finally
        {
            if (buffer != 0)
            {
                dhara_bytes_free(buffer, length);
            }
        }
    }

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    private static unsafe partial int dhara_write_file(
        string path,
        byte* dataPtr,
        nuint dataLen,
        out nint outPathPtr,
        out nuint outPathLen,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    private static partial int dhara_copy_file(
        string source,
        string destination,
        out nint outPathPtr,
        out nuint outPathLen,
        out nint errorPtr,
        out nuint errorLen);

    public static unsafe void WriteBytes(string path, byte[] data)
    {
        fixed (byte* ptr = data)
        {
            var code = dhara_write_file(path, ptr, (nuint)data.Length, out var outPath, out var outLen, out var errPtr, out var errLen);
            FreeError(errPtr, errLen);
            if (outPath != 0)
            {
                dhara_string_free(outPath, outLen);
            }

            if (code != 0)
            {
                throw new InvalidOperationException($"dhara_write_file failed: {code}");
            }
        }
    }

    public static void CopyFile(string source, string destination)
    {
        var code = dhara_copy_file(source, destination, out var outPath, out var outLen, out var errPtr, out var errLen);
        FreeError(errPtr, errLen);
        if (outPath != 0)
        {
            dhara_string_free(outPath, outLen);
        }

        if (code != 0)
        {
            throw new InvalidOperationException($"dhara_copy_file failed: {code}");
        }
    }

    private static void FreeError(nint errPtr, nuint errLen)
    {
        if (errPtr != 0)
        {
            dhara_string_free(errPtr, errLen);
        }
    }
}
