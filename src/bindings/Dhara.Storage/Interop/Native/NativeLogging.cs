using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Dhara.Storage.Core;

namespace Dhara.Storage.Interop.Native;

internal static unsafe partial class NativeLogging
{
    private const string LibraryName = NativeMemory.LibraryName;

    private static bool _registered;

    [LibraryImport(LibraryName)]
    private static partial NativeStatus dhara_register_logger(
        delegate* unmanaged[Cdecl]<byte*, nuint, nint, void> callback,
        nint userData,
        out nint errorPtr,
        out nuint errorLen);

    [LibraryImport(LibraryName)]
    private static partial NativeStatus dhara_unregister_logger(out nint errorPtr, out nuint errorLen);

    internal static void RegisterLogger()
    {
        NativeHelpers.EnsureSupportedPlatform();
        if (_registered)
        {
            return;
        }

        var status = dhara_register_logger(&OnNativeLog, 0, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        _registered = true;
    }

    internal static void UnregisterLogger()
    {
        if (!_registered)
        {
            return;
        }

        var status = dhara_unregister_logger(out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        _registered = false;
    }

    [UnmanagedCallersOnly(CallConvs = [typeof(CallConvCdecl)])]
    private static void OnNativeLog(byte* dataPtr, nuint dataLen, nint _)
    {
        try
        {
            var json = NativeMemory.ReadUtf8((nint)dataPtr, dataLen);
            if (string.IsNullOrWhiteSpace(json))
            {
                return;
            }

            var record = NativeJson.Deserialize<NativeLogRecordDto>(json);
            DharaStorageLogBridge.LogNative(record);
        }
        catch
        {
            // Native logging must never throw back across the FFI boundary.
        }
    }
}
