using Dhara.Storage.Abstractions;
using Dhara.Storage.Core;
using Dhara.Storage.Interop.Handles;
using Dhara.Storage.Interop.Native;
using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;
using Dhara.Storage.Models.Progress;

namespace Dhara.Storage;

/// <summary>
/// Represents a path-based file wrapper backed by the native Dhara Storage runtime.
/// </summary>
public sealed class StorageFile : StorageItemBase, IStorageFile
{
    private FileInformation? _cachedInformation;
    private FileInformation? _cachedInformationWithAnalysis;

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageFile"/> class.
    /// </summary>
    public StorageFile(string path) : base(path)
    {
    }

    /// <inheritdoc />
    public override bool Exists => File.Exists(FullPath);

    /// <inheritdoc />
    public FileInformation Information => _cachedInformation ??= NativeQueryInvoker.GetFileInformation(FullPath, includeAnalysis: false);

    /// <inheritdoc />
    public FileInformation RefreshInformation(bool includeAnalysis = false)
    {
        EnsureNotDisposed();
        var info = NativeQueryInvoker.GetFileInformation(FullPath, includeAnalysis);
        if (includeAnalysis)
        {
            _cachedInformationWithAnalysis = info;
            _cachedInformation = info with { Analysis = null };
        }
        else
        {
            _cachedInformation = info;
        }

        return info;
    }

    /// <inheritdoc />
    public AnalysisReport Analyze()
    {
        EnsureNotDisposed();
        return NativeQueryInvoker.AnalyzePath(FullPath);
    }

    /// <inheritdoc />
    public byte[] ReadBytes()
    {
        EnsureNotDisposed();
        return NativeQueryInvoker.ReadFileBytes(FullPath);
    }

    /// <inheritdoc />
    public Task<byte[]> ReadBytesAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_read_file(FullPath, out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        return NativeOperationRunner.RunAsync(handle, static operation => operation.TakeBytesResult(), progress, cancellationToken);
    }

    /// <inheritdoc />
    public string ReadText()
    {
        EnsureNotDisposed();
        return NativeQueryInvoker.ReadFileText(FullPath);
    }

    /// <inheritdoc />
    public Task<string> ReadTextAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_read_file_text(FullPath, out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        return NativeOperationRunner.RunAsync(handle, static operation => operation.TakeStringResult(), progress, cancellationToken);
    }

    /// <inheritdoc />
    public void Write(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true) =>
        WriteAsync(content, progress, overwrite, createParentDirectories).GetAwaiter().GetResult();

    /// <inheritdoc />
    public unsafe Task WriteAsync(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();

        if (progress is null)
        {
            NativeQueryInvoker.WriteFileBytes(FullPath, content);
            InvalidateCaches();
            return Task.CompletedTask;
        }

        NativeOperationHandle handle;
        fixed (byte* ptr = content)
        {
            var status = NativeOperations.dhara_operation_start_write_file(
                FullPath,
                ptr,
                (nuint)content.Length,
                NativeHelpers.ToNativeBool(overwrite),
                NativeHelpers.ToNativeBool(createParentDirectories),
                out var nativeHandle,
                out var errorPtr,
                out var errorLen);
            handle = NativeOperationHandle.Create(status, nativeHandle, errorPtr, errorLen);
        }

        return AwaitWriteCompletionAsync(handle, progress, cancellationToken);
    }

    /// <inheritdoc />
    public void WriteText(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true) =>
        WriteTextAsync(text, progress, overwrite, createParentDirectories).GetAwaiter().GetResult();

    /// <inheritdoc />
    public Task WriteTextAsync(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();

        if (progress is null)
        {
            NativeQueryInvoker.WriteFileText(FullPath, text);
            InvalidateCaches();
            return Task.CompletedTask;
        }

        var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_write_file_text(
                FullPath,
                text,
                NativeHelpers.ToNativeBool(overwrite),
                NativeHelpers.ToNativeBool(createParentDirectories),
                out var nativeHandle,
                out var errorPtr,
                out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });

        return AwaitWriteCompletionAsync(handle, progress, cancellationToken);
    }

    /// <inheritdoc />
    public async Task WriteAsync(Stream stream, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        ArgumentNullException.ThrowIfNull(stream);

        using var session = NativeWriteSessionHandle.Create(FullPath, overwrite, createParentDirectories);
        var buffer = new byte[64 * 1024];
        ulong transferred = 0;
        var total = stream.CanSeek ? (ulong?)stream.Length : null;

        try
        {
            while (true)
            {
                cancellationToken.ThrowIfCancellationRequested();
                var read = await stream.ReadAsync(buffer.AsMemory(), cancellationToken).ConfigureAwait(false);
                if (read == 0)
                {
                    break;
                }

                session.WriteChunk(buffer.AsSpan(0, read));
                transferred += (ulong)read;
                progress?.Report(new StorageProgress(total, transferred, 0));
            }

            session.Complete();
            InvalidateCaches();
        }
        catch
        {
            session.Abort();
            throw;
        }
    }

    /// <inheritdoc />
    public IStorageFile Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false) =>
        CopyAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task<IStorageFile> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        if (progress is null)
        {
            return new StorageFile(NativeQueryInvoker.CopyFile(FullPath, destination));
        }

        var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_copy_file(FullPath, destination, NativeHelpers.ToNativeBool(overwrite), out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });

        var newPath = await NativeOperationRunner.RunAsync(handle, static operation => operation.TakeStringResult(), progress, cancellationToken).ConfigureAwait(false);
        return new StorageFile(newPath);
    }

    /// <inheritdoc />
    public void Move(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false) =>
        MoveAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task MoveAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();

        string newPath;
        if (progress is null)
        {
            newPath = NativeQueryInvoker.MoveFile(FullPath, destination);
        }
        else
        {
            var handle = NativeOperationHandle.Create(() =>
            {
                var status = NativeOperations.dhara_operation_start_move_file(FullPath, destination, NativeHelpers.ToNativeBool(overwrite), out var nativeHandle, out var errorPtr, out var errorLen);
                return (status, nativeHandle, errorPtr, errorLen);
            });
            newPath = await NativeOperationRunner.RunAsync(handle, static operation => operation.TakeStringResult(), progress, cancellationToken).ConfigureAwait(false);
        }

        UpdatePath(newPath);
    }

    /// <inheritdoc />
    public void Rename(string newName) => RenameAsync(newName).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task RenameAsync(string newName, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        using var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_rename_file(FullPath, newName, out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        await handle.WaitForCompletionAsync(null, cancellationToken).ConfigureAwait(false);
        UpdatePath(handle.TakeStringResult());
    }

    /// <inheritdoc />
    public void Delete() => DeleteAsync().GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task DeleteAsync(CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        using var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_delete_file(FullPath, out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        await handle.WaitForCompletionAsync(null, cancellationToken).ConfigureAwait(false);
        InvalidateCaches();
    }

    /// <inheritdoc />
    protected override void InvalidateCaches()
    {
        _cachedInformation = null;
        _cachedInformationWithAnalysis = null;
    }

    private async Task AwaitWriteCompletionAsync(
        NativeOperationHandle handle,
        IProgress<StorageProgress>? progress,
        CancellationToken cancellationToken)
    {
        using (handle)
        {
            await handle.WaitForCompletionAsync(progress, cancellationToken).ConfigureAwait(false);
            handle.TakeStringResult();
            InvalidateCaches();
        }
    }
}
