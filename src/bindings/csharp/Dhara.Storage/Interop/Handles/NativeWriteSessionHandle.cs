using Dhara.Storage.Interop.Native;

namespace Dhara.Storage.Interop.Handles;

internal sealed class NativeWriteSessionHandle : IDisposable
{
    private nint _handle;

    private NativeWriteSessionHandle(nint handle)
    {
        _handle = handle;
    }

    internal static NativeWriteSessionHandle Create(string path, bool overwrite, bool createParentDirectories)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeSessions.dhara_write_session_create(
            path,
            NativeHelpers.ToNativeBool(overwrite),
            NativeHelpers.ToNativeBool(createParentDirectories),
            out var handle,
            out var errorPtr,
            out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return new NativeWriteSessionHandle(handle);
    }

    internal unsafe void WriteChunk(ReadOnlySpan<byte> chunk)
    {
        ThrowIfDisposed();
        NativeHelpers.EnsureSupportedPlatform();
        fixed (byte* ptr = chunk)
        {
            var status = NativeSessions.dhara_write_session_write_chunk(_handle, ptr, (nuint)chunk.Length, out var errorPtr, out var errorLen);
            NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        }
    }

    internal string Complete()
    {
        ThrowIfDisposed();
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeSessions.dhara_write_session_complete(_handle, out var pathPtr, out var pathLen, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return NativeMemory.ReadUtf8AndFree(pathPtr, pathLen);
    }

    internal void Abort()
    {
        if (_handle == 0)
        {
            return;
        }

        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeSessions.dhara_write_session_abort(_handle, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
    }

    public void Dispose()
    {
        if (_handle == 0)
        {
            return;
        }

        NativeHelpers.EnsureSupportedPlatform();
        NativeSessions.dhara_write_session_free(_handle);
        _handle = 0;
        GC.SuppressFinalize(this);
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_handle == 0, this);
    }
}
