using System.Runtime.InteropServices;

namespace Dhara.Storage.Interop.Native;

internal static partial class NativeSessions
{
    private const string LibraryName = NativeMemory.LibraryName;

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_write_session_create(string path, byte overwrite, byte createParentDirectories, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static unsafe partial NativeStatus dhara_write_session_write_chunk(nint handle, byte* dataPtr, nuint dataLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_write_session_complete(nint handle, out nint pathPtr, out nuint pathLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_write_session_abort(nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial void dhara_write_session_free(nint handle);
}
