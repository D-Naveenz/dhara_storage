using System.Runtime.InteropServices;

namespace Dhara.Storage.Interop.Native;

internal static partial class NativeOperations
{
    private const string LibraryName = NativeMemory.LibraryName;

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_copy_file(string source, string destination, byte overwrite, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_move_file(string source, string destination, byte overwrite, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_rename_file(string source, string newName, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_delete_file(string path, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_read_file(string path, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_read_file_text(string path, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static unsafe partial NativeStatus dhara_operation_start_write_file(string path, byte* dataPtr, nuint dataLen, byte overwrite, byte createParentDirectories, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_write_file_text(string path, string text, byte overwrite, byte createParentDirectories, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_create_directory(string path, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_create_directory_all(string path, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_copy_directory(string source, string destination, byte overwrite, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_move_directory(string source, string destination, byte overwrite, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_rename_directory(string source, string newName, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_operation_start_delete_directory(string path, byte recursive, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_operation_get_snapshot(nint handle, out NativeOperationSnapshot snapshot, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_operation_cancel(nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_operation_take_string_result(nint handle, out nint stringPtr, out nuint stringLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_operation_take_bytes_result(nint handle, out nint bytesPtr, out nuint bytesLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_operation_get_error(nint handle, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial void dhara_operation_free(nint handle);
}
