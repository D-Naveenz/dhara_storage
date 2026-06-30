using System.Runtime.InteropServices;

namespace Dhara.Storage.Interop.Native;

internal static partial class NativeWatching
{
    private const string LibraryName = NativeMemory.LibraryName;

    [LibraryImport(LibraryName, StringMarshalling = StringMarshalling.Utf8)]
    internal static partial NativeStatus dhara_watch_create(string path, byte recursive, ulong debounceWindowMs, out nint handle, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_try_recv_event(nint handle, out nint eventPtr, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_recv_event(nint handle, out nint eventPtr, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_recv_event_timeout(nint handle, ulong timeoutMs, out nint eventPtr, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName)]
    internal static partial void dhara_watch_event_free(nint eventPtr);

    [LibraryImport(LibraryName)]
    internal static partial NativeStatus dhara_watch_stop(nint handle, out nint errorPtr, out nuint errorLen);

    [LibraryImport(LibraryName)]
    internal static partial void dhara_watch_free(nint handle);
}
