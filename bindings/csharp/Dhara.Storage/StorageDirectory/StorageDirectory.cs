using Dhara.Storage.Abstractions;
using Dhara.Storage.Core;
using Dhara.Storage.Models.Metadata;
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
    private DirectoryMetadata? _cachedMetadata;
    private DirectoryMetadata? _cachedMetadataWithSummary;
    private CancellationTokenSource? _watchCancellationSource;
    private AsyncServerStreamingCall<WatchEvent>? _watchCall;
    private Task? _watchLoopTask;

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageDirectory"/> class.
    /// </summary>
    /// <param name="path">Directory path to wrap; may point to an existing directory or a future create destination.</param>
    public StorageDirectory(string path) : base(path)
    {
    }

    /// <inheritdoc />
    public override bool Exists => Directory.Exists(AbsolutePath);

    /// <inheritdoc />
    public StorageSize Size()
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = DaemonClient.Call(
            (client, options) => client.GetDirectoryMetadata(
                new GetDirectoryMetadataRequest { Path = path, IncludeSummary = true },
                options),
            path,
            nameof(DharaSd.DharaSdClient.GetDirectoryMetadata));
        return response.Size is null
            ? new StorageSize(0, "0 B")
            : DaemonModelFactory.ToStorageSize(response.Size);
    }

    /// <inheritdoc />
    public DirectoryMetadata Metadata => _cachedMetadata ??= LoadMetadata(includeSummary: false);

    /// <inheritdoc />
    public event EventHandler<StorageChangedEventArgs>? Changed;

    /// <inheritdoc />
    public bool IsWatching => _watchLoopTask is not null && !_watchLoopTask.IsCompleted;

    /// <inheritdoc />
    public DirectoryMetadata RefreshMetadata(bool includeSummary = false)
    {
        EnsureNotDisposed();
        var info = LoadMetadata(includeSummary);
        if (includeSummary)
        {
            _cachedMetadataWithSummary = info;
            _cachedMetadata = info with { Summary = null };
        }
        else
        {
            _cachedMetadata = info;
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
        return new StorageFile(Path.GetFullPath(Path.Combine(AbsolutePath, relativePath)));
    }

    /// <inheritdoc />
    public StorageDirectory GetDirectory(string relativePath)
    {
        EnsureNotDisposed();
        return new StorageDirectory(Path.GetFullPath(Path.Combine(AbsolutePath, relativePath)));
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
    public void Copy(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false) =>
        CopyAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public StorageProcess CopyAsync(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var source = AbsolutePath;
        return DaemonClient.StartCopyProcess(
            (client, options) => client.CopyDirectory(new CopyDirectoryRequest { Source = source, Destination = destination, Overwrite = overwrite }, options),
            progress,
            source,
            nameof(DharaSd.DharaSdClient.CopyDirectory),
            cancellationToken);
    }

    /// <inheritdoc />
    public void Move(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false) =>
        MoveAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public StorageProcess MoveAsync(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var source = AbsolutePath;
        return StorageProcess.Start(async ct =>
        {
            _ = progress;
            var response = await DaemonClient.CallAsync(
                (client, options) => client.MovePathAsync(new MovePathRequest { Source = source, Destination = destination, Overwrite = overwrite }, options),
                ct,
                source,
                nameof(DharaSd.DharaSdClient.MovePath)).ConfigureAwait(false);
            UpdatePathFromDestination(destination, response.Path);
            return response.Path;
        }, cancellationToken);
    }

    /// <inheritdoc />
    public void Rename(string newName) => RenameAsync(newName).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task RenameAsync(string newName, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.RenamePathAsync(new RenamePathRequest { Path = path, NewName = newName }, options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.RenamePath)).ConfigureAwait(false);
        UpdatePathAbsolute(response.Path);
    }

    /// <inheritdoc />
    public void Delete(bool recursive = true) => DeleteAsync(recursive).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task DeleteAsync(bool recursive = true, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
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

        var path = AbsolutePath;
        _watchCancellationSource = new CancellationTokenSource();
        var cancellationToken = _watchCancellationSource.Token;

        // Open the stream and wait for response headers so the daemon has attached
        // notify before this method returns (matches prior in-process Create semantics).
        var call = DaemonClient.Client.WatchDirectory(
            new WatchDirectoryRequest
            {
                Path = path,
                Recursive = options.Recursive,
                DebounceWindowMs = (uint)Math.Clamp(options.DebounceWindow.TotalMilliseconds, 1, uint.MaxValue),
            },
            new CallOptions(cancellationToken: cancellationToken));
        call.ResponseHeadersAsync.GetAwaiter().GetResult();

        _watchCall = call;
        _watchLoopTask = Task.Run(() => WatchLoopAsync(call, cancellationToken), cancellationToken);
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
        catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
        {
        }
        finally
        {
            _watchLoopTask = null;
            _watchCall = null;
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
        _cachedMetadata = null;
        _cachedMetadataWithSummary = null;
    }

    private DirectoryMetadata LoadMetadata(bool includeSummary)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = DaemonClient.Call(
            (client, options) => client.GetDirectoryMetadata(
                new GetDirectoryMetadataRequest
                {
                    Path = path,
                    IncludeSummary = includeSummary,
                },
                options),
            path,
            nameof(DharaSd.DharaSdClient.GetDirectoryMetadata));
        return DaemonModelFactory.ToDirectoryMetadata(response);
    }

    private IReadOnlyList<StorageEntry> ListEntries(bool recursive)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = DaemonClient.Call(
            (client, options) => client.ListEntries(new ListEntriesRequest { Path = path, Recursive = recursive }, options),
            path,
            nameof(DharaSd.DharaSdClient.ListEntries));
        return response.Entries.Select(DaemonModelFactory.ToStorageEntry).ToArray();
    }

    private async Task<StorageDirectory> CreateCoreAsync(bool createParents, CancellationToken cancellationToken)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.CreateDirectoryAsync(new CreateDirectoryRequest { Path = path, CreateParents = createParents }, options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.CreateDirectory)).ConfigureAwait(false);
        RefreshResolvedPath(response.Path);
        return this;
    }

    private async Task WatchLoopAsync(
        AsyncServerStreamingCall<WatchEvent> call,
        CancellationToken cancellationToken)
    {
        try
        {
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
        finally
        {
            call.Dispose();
            if (ReferenceEquals(_watchCall, call))
            {
                _watchCall = null;
            }
        }
    }
}
