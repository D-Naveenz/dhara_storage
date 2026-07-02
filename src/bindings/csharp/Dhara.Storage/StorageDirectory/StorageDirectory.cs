using Dhara.Storage.Abstractions;
using Dhara.Storage.Core;
using Dhara.Storage.Interop.Handles;
using Dhara.Storage.Interop.Native;
using Dhara.Storage.Models.Information;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Models.Watching;

namespace Dhara.Storage;

/// <summary>
/// Represents a path-based directory wrapper backed by the native Dhara Storage runtime.
/// </summary>
public sealed class StorageDirectory : StorageItemBase, IStorageDirectory
{
    private DirectoryInformation? _cachedInformation;
    private DirectoryInformation? _cachedInformationWithSummary;
    private NativeWatchHandle? _watchHandle;
    private CancellationTokenSource? _watchCancellationSource;
    private Task? _watchLoopTask;

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageDirectory"/> class.
    /// </summary>
    public StorageDirectory(string path) : base(path)
    {
    }

    /// <inheritdoc />
    public override bool Exists => Directory.Exists(FullPath);

    /// <inheritdoc />
    public DirectoryInformation Information => _cachedInformation ??= NativeQueryInvoker.GetDirectoryInformation(FullPath, includeSummary: false);

    /// <inheritdoc />
    public event EventHandler<StorageChangedEventArgs>? Changed;

    /// <inheritdoc />
    public bool IsWatching => _watchLoopTask is not null && !_watchLoopTask.IsCompleted;

    /// <inheritdoc />
    public DirectoryInformation RefreshInformation(bool includeSummary = false)
    {
        EnsureNotDisposed();
        var info = NativeQueryInvoker.GetDirectoryInformation(FullPath, includeSummary);
        if (includeSummary)
        {
            _cachedInformationWithSummary = info;
            _cachedInformation = info with { Summary = null };
        }
        else
        {
            _cachedInformation = info;
        }

        return info;
    }

    /// <inheritdoc />
    public IReadOnlyList<StorageFile> GetFiles(bool recursive = false)
    {
        EnsureNotDisposed();
        return NativeQueryInvoker.ListFiles(FullPath, recursive)
            .Select(static entry => new StorageFile(entry.Path))
            .ToArray();
    }

    /// <inheritdoc />
    public IReadOnlyList<StorageDirectory> GetDirectories(bool recursive = false)
    {
        EnsureNotDisposed();
        return NativeQueryInvoker.ListDirectories(FullPath, recursive)
            .Select(static entry => new StorageDirectory(entry.Path))
            .ToArray();
    }

    /// <inheritdoc />
    public IReadOnlyList<StorageEntry> GetEntries(bool recursive = false)
    {
        EnsureNotDisposed();
        return NativeQueryInvoker.ListEntries(FullPath, recursive);
    }

    /// <inheritdoc />
    public StorageFile GetFile(string relativePath)
    {
        EnsureNotDisposed();
        return new StorageFile(Path.GetFullPath(Path.Combine(FullPath, relativePath)));
    }

    /// <inheritdoc />
    public StorageDirectory GetDirectory(string relativePath)
    {
        EnsureNotDisposed();
        return new StorageDirectory(Path.GetFullPath(Path.Combine(FullPath, relativePath)));
    }

    /// <inheritdoc />
    public StorageDirectory Create()
    {
        EnsureNotDisposed();
        UpdatePath(NativeQueryInvoker.CreateDirectory(FullPath));
        return this;
    }

    /// <inheritdoc />
    public StorageDirectory CreateAll()
    {
        EnsureNotDisposed();
        UpdatePath(NativeQueryInvoker.CreateDirectoryAll(FullPath));
        return this;
    }

    /// <inheritdoc />
    public async Task<StorageDirectory> CreateAsync(CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        using var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_create_directory(FullPath, out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        await handle.WaitForCompletionAsync(null, cancellationToken).ConfigureAwait(false);
        UpdatePath(handle.TakeStringResult());
        return this;
    }

    /// <inheritdoc />
    public async Task<StorageDirectory> CreateAllAsync(CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        using var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_create_directory_all(FullPath, out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        await handle.WaitForCompletionAsync(null, cancellationToken).ConfigureAwait(false);
        UpdatePath(handle.TakeStringResult());
        return this;
    }

    /// <inheritdoc />
    public IStorageDirectory Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false) =>
        CopyAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task<IStorageDirectory> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        if (progress is null)
        {
            return new StorageDirectory(NativeQueryInvoker.CopyDirectory(FullPath, destination));
        }

        var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_copy_directory(FullPath, destination, NativeHelpers.ToNativeBool(overwrite), out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });

        var newPath = await NativeOperationRunner.RunAsync(handle, static operation => operation.TakeStringResult(), progress, cancellationToken).ConfigureAwait(false);
        return new StorageDirectory(newPath);
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
            newPath = NativeQueryInvoker.MoveDirectory(FullPath, destination);
        }
        else
        {
            var handle = NativeOperationHandle.Create(() =>
            {
                var status = NativeOperations.dhara_operation_start_move_directory(FullPath, destination, NativeHelpers.ToNativeBool(overwrite), out var nativeHandle, out var errorPtr, out var errorLen);
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
            var status = NativeOperations.dhara_operation_start_rename_directory(FullPath, newName, out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        await handle.WaitForCompletionAsync(null, cancellationToken).ConfigureAwait(false);
        UpdatePath(handle.TakeStringResult());
    }

    /// <inheritdoc />
    public void Delete(bool recursive = true) => DeleteAsync(recursive).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task DeleteAsync(bool recursive = true, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        using var handle = NativeOperationHandle.Create(() =>
        {
            var status = NativeOperations.dhara_operation_start_delete_directory(FullPath, NativeHelpers.ToNativeBool(recursive), out var nativeHandle, out var errorPtr, out var errorLen);
            return (status, nativeHandle, errorPtr, errorLen);
        });
        await handle.WaitForCompletionAsync(null, cancellationToken).ConfigureAwait(false);
        InvalidateCaches();
    }

    /// <inheritdoc />
    public void StartWatching(StorageWatchOptions? options = null)
    {
        EnsureNotDisposed();
        options ??= new StorageWatchOptions();
        if (IsWatching)
        {
            return;
        }

        _watchHandle = NativeWatchHandle.Create(FullPath, options);
        _watchCancellationSource = new CancellationTokenSource();
        _watchLoopTask = Task.Run(() => WatchLoopAsync(_watchHandle, options, _watchCancellationSource.Token));
    }

    /// <inheritdoc />
    public void StopWatching()
    {
        _watchCancellationSource?.Cancel();
        _watchHandle?.Stop();

        try
        {
            _watchLoopTask?.GetAwaiter().GetResult();
        }
        catch (OperationCanceledException)
        {
        }
        finally
        {
            _watchLoopTask = null;
            _watchCancellationSource?.Dispose();
            _watchCancellationSource = null;
            _watchHandle?.Dispose();
            _watchHandle = null;
        }
    }

    /// <inheritdoc />
    public override void Dispose()
    {
        StopWatching();
        base.Dispose();
    }

    /// <inheritdoc />
    protected override void InvalidateCaches()
    {
        _cachedInformation = null;
        _cachedInformationWithSummary = null;
    }

    private async Task WatchLoopAsync(NativeWatchHandle handle, StorageWatchOptions options, CancellationToken cancellationToken)
    {
        try
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                var change = handle.ReceiveTimeout(options.ReceiveTimeout);
                if (change is null)
                {
                    await Task.Delay(25, CancellationToken.None).ConfigureAwait(false);
                    continue;
                }

                InvalidateCaches();
                Changed?.Invoke(this, change);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }
}
