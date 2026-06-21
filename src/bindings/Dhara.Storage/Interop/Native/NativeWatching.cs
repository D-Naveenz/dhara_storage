using System.Runtime.InteropServices;

namespace Dhara.Storage.Interop.Native;

internal static partial class NativeWatching
{
    private const string LibraryName = NativeMemory.LibraryName;

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_watch_create(string path, byte recursive, ulong debounceWindowMs, out nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_try_recv_json(nint handle, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_recv_json(nint handle, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_recv_json_timeout(nint handle, ulong timeoutMs, out nint jsonPtr, out nuint jsonLen, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_stop(nint handle, out nint errorPtr, out nuint errorLen);
    [LibraryImport(LibraryName)]
    internal static partial void dhara_watch_free(nint handle);
}
