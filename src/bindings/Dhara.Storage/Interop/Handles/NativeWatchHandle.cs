using Dhara.Storage.Interop.Native;
using Dhara.Storage.Models.Watching;

namespace Dhara.Storage.Interop.Handles;

internal sealed class NativeWatchHandle : IDisposable
{
    private nint _handle;

    private NativeWatchHandle(nint handle)
    {
        _handle = handle;
    }

    internal static NativeWatchHandle Create(string path, StorageWatchOptions options)
    {
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeWatching.dhara_watch_create(
            path,
            NativeHelpers.ToNativeBool(options.Recursive),
            (ulong)Math.Max(1, options.DebounceWindow.TotalMilliseconds),
            out var handle,
            out var errorPtr,
            out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        return new NativeWatchHandle(handle);
    }

    internal StorageChangedEventArgs? ReceiveTimeout(TimeSpan timeout)
    {
        ThrowIfDisposed();
        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeWatching.dhara_watch_recv_event_timeout_v2(_handle, (ulong)Math.Max(0, timeout.TotalMilliseconds), out var eventPtr, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
        if (eventPtr == 0)
        {
            return null;
        }

        try
        {
            return NativeTyped.ToWatchEvent(eventPtr);
        }
        finally
        {
            NativeWatching.dhara_watch_event_free(eventPtr);
        }
    }

    internal void Stop()
    {
        if (_handle == 0)
        {
            return;
        }

        NativeHelpers.EnsureSupportedPlatform();
        var status = NativeWatching.dhara_watch_stop(_handle, out var errorPtr, out var errorLen);
        NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
    }

    public void Dispose()
    {
        if (_handle == 0)
        {
            return;
        }

        NativeHelpers.EnsureSupportedPlatform();
        NativeWatching.dhara_watch_free(_handle);
        _handle = 0;
        GC.SuppressFinalize(this);
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_handle == 0, this);
    }
}
