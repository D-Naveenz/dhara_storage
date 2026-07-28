using Dhara.Storage.Abstractions;
using Dhara.Storage.Core;
using Dhara.Storage.Models.Information;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Models.Watching;
using Dhara.Storage.Runtime;
using Dhara.Storage.Sd.V1;
using Grpc.Core;

namespace Dhara.Storage;

/// <summary>
/// Path-based directory wrapper backed by the <c>dhara-sd</c> sidecar daemon.
/// </summary>
/// <remarks>
/// Prefer <see cref="DharaStorage.Directory"/> from application code. Supports listing,
/// transfers, and optional debounced watching via <see cref="Changed"/>. Watching consumes a
/// server-streaming gRPC call for the lifetime of <see cref="StartWatching"/>.
/// </remarks>
public sealed class StorageDirectory : StorageItemBase, IStorageDirectory
{
    private DirectoryInformation? _cachedInformation;
    private DirectoryInformation? _cachedInformationWithSummary;
    private CancellationTokenSource? _watchCancellationSource;
    private Task? _watchLoopTask;

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageDirectory"/> class.
    /// </summary>
    /// <param name="path">Directory path to wrap; may point to an existing directory or a future create destination.</param>
    public StorageDirectory(string path) : base(path)
    {
    }

    /// <inheritdoc />
    public override bool Exists => Directory.Exists(FullPath);

    /// <inheritdoc />
    public DirectoryInformation Information => _cachedInformation ??= LoadInformation(includeSummary: false);

    /// <inheritdoc />
    public event EventHandler<StorageChangedEventArgs>? Changed;

    /// <inheritdoc />
    public bool IsWatching => _watchLoopTask is not null && !_watchLoopTask.IsCompleted;

    /// <inheritdoc />
    public DirectoryInformation RefreshInformation(bool includeSummary = false)
    {
        EnsureNotDisposed();
        var info = LoadInformation(includeSummary);
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
    public IReadOnlyList<StorageFile> GetFiles(bool recursive = false) =>
        ListEntries(recursive).Where(static entry => !entry.IsDirectory).Select(static entry => new StorageFile(entry.Path)).ToArray();

    /// <inheritdoc />
    public IReadOnlyList<StorageDirectory> GetDirectories(bool recursive = false) =>
        ListEntries(recursive).Where(static entry => entry.IsDirectory).Select(static entry => new StorageDirectory(entry.Path)).ToArray();

    /// <inheritdoc />
    public IReadOnlyList<StorageEntry> GetEntries(bool recursive = false) => ListEntries(recursive);

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
    public StorageDirectory Create() => CreateAsync().GetAwaiter().GetResult();

    /// <inheritdoc />
    public StorageDirectory CreateAll() => CreateAllAsync().GetAwaiter().GetResult();

    /// <inheritdoc />
    public Task<StorageDirectory> CreateAsync(CancellationToken cancellationToken = default) =>
        CreateCoreAsync(createParents: false, cancellationToken);

    /// <inheritdoc />
    public Task<StorageDirectory> CreateAllAsync(CancellationToken cancellationToken = default) =>
        CreateCoreAsync(createParents: true, cancellationToken);

    /// <inheritdoc />
    public IStorageDirectory Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false) =>
        CopyAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task<IStorageDirectory> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var source = FullPath;
        var newPath = await DaemonClient.ConsumeCopyProgressAsync(
            (client, options) => client.CopyDirectory(new CopyDirectoryRequest { Source = source, Destination = destination, Overwrite = overwrite }, options),
            progress,
            source,
            nameof(DharaSd.DharaSdClient.CopyDirectory),
            cancellationToken).ConfigureAwait(false);
        return new StorageDirectory(newPath);
    }

    /// <inheritdoc />
    public void Move(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false) =>
        MoveAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task MoveAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var source = FullPath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.MovePathAsync(new MovePathRequest { Source = source, Destination = destination, Overwrite = overwrite }, options),
            cancellationToken,
            source,
            nameof(DharaSd.DharaSdClient.MovePath)).ConfigureAwait(false);
        UpdatePath(response.Path);
    }

    /// <inheritdoc />
    public void Rename(string newName) => RenameAsync(newName).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task RenameAsync(string newName, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var path = FullPath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.RenamePathAsync(new RenamePathRequest { Path = path, NewName = newName }, options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.RenamePath)).ConfigureAwait(false);
        UpdatePath(response.Path);
    }

    /// <inheritdoc />
    public void Delete(bool recursive = true) => DeleteAsync(recursive).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task DeleteAsync(bool recursive = true, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var path = FullPath;
        await DaemonClient.CallAsync(
            (client, options) => client.DeletePathAsync(new DeletePathRequest { Path = path, Recursive = recursive }, options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.DeletePath)).ConfigureAwait(false);
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

        _watchCancellationSource = new CancellationTokenSource();
        _watchLoopTask = Task.Run(() => WatchLoopAsync(options, _watchCancellationSource.Token));
    }

    /// <inheritdoc />
    public void StopWatching()
    {
        _watchCancellationSource?.Cancel();

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

    private DirectoryInformation LoadInformation(bool includeSummary)
    {
        EnsureNotDisposed();
        var path = FullPath;
        var response = DaemonClient.Call(
            (client, options) => client.GetDirectoryInfo(new GetDirectoryInfoRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.GetDirectoryInfo));
        var summary = includeSummary ? DaemonModelFactory.BuildDirectorySummary(path) : null;
        return DaemonModelFactory.ToDirectoryInformation(response, summary);
    }

    private IReadOnlyList<StorageEntry> ListEntries(bool recursive)
    {
        EnsureNotDisposed();
        var path = FullPath;
        var response = DaemonClient.Call(
            (client, options) => client.ListEntries(new ListEntriesRequest { Path = path, Recursive = recursive }, options),
            path,
            nameof(DharaSd.DharaSdClient.ListEntries));
        return response.Entries.Select(DaemonModelFactory.ToStorageEntry).ToArray();
    }

    private async Task<StorageDirectory> CreateCoreAsync(bool createParents, CancellationToken cancellationToken)
    {
        EnsureNotDisposed();
        var path = FullPath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.CreateDirectoryAsync(new CreateDirectoryRequest { Path = path, CreateParents = createParents }, options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.CreateDirectory)).ConfigureAwait(false);
        UpdatePath(response.Path);
        return this;
    }

    private async Task WatchLoopAsync(StorageWatchOptions options, CancellationToken cancellationToken)
    {
        var path = FullPath;
        try
        {
            using var call = DaemonClient.Client.WatchDirectory(
                new WatchDirectoryRequest
                {
                    Path = path,
                    Recursive = options.Recursive,
                    DebounceWindowMs = (uint)Math.Clamp(options.DebounceWindow.TotalMilliseconds, 1, uint.MaxValue),
                },
                new CallOptions(cancellationToken: cancellationToken));

            while (await call.ResponseStream.MoveNext(cancellationToken).ConfigureAwait(false))
            {
                InvalidateCaches();
                Changed?.Invoke(this, DaemonModelFactory.ToChangedEventArgs(call.ResponseStream.Current));
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
        {
        }
    }
}
