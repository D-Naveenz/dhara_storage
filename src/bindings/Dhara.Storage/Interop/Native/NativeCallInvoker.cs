namespace Dhara.Storage.Interop.Native;

internal static class NativeCallInvoker
{
    internal delegate NativeStatus NativeDataCall(out nint dataPtr, out nuint dataLen, out nint errorPtr, out nuint errorLen);
    internal delegate NativeStatus NativeUnitCall(out nint errorPtr, out nuint errorLen);

    internal static T InvokeJson<T>(NativeDataCall call) =>
        NativeJson.Deserialize<T>(InvokeString(call));

    internal static string InvokeString(NativeDataCall call)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = call(out var dataPtr, out var dataLen, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return NativeMemory.ReadUtf8AndFree(dataPtr, dataLen);
    }

    internal static byte[] InvokeBytes(NativeDataCall call)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = call(out var dataPtr, out var dataLen, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return NativeMemory.ReadBytesAndFree(dataPtr, dataLen);
    }

    internal static void InvokeUnit(NativeUnitCall call)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = call(out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
    }
}
