using Dhara.Storage.Abstractions;
using Dhara.Storage.Core;
using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Metadata;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Runtime;
using Dhara.Storage.Sd.V1;
using Microsoft.Win32.SafeHandles;

namespace Dhara.Storage;

/// <summary>
/// Path-based file wrapper backed by the <c>dhara-sd</c> sidecar daemon.
/// </summary>
/// <remarks>
/// Prefer <see cref="DharaStorage.File"/> from application code. The wrapper caches
/// metadata snapshots; call <see cref="RefreshMetadata"/> after external changes. Reads and
/// writes use OS handle transfer (the daemon duplicates a native file handle into this process)
/// rather than sending bytes over gRPC.
/// </remarks>
public sealed class StorageFile : StorageItemBase, IStorageFile
{
    private const int StreamBufferSize = 64 * 1024;

    private FileMetadata? _cachedMetadata;
    private FileMetadata? _cachedMetadataWithAnalysis;

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageFile"/> class.
    /// </summary>
    /// <param name="path">File path to wrap; may point to an existing file or a future write destination.</param>
    public StorageFile(string path) : base(path)
    {
    }

    /// <inheritdoc />
    public override bool Exists => File.Exists(AbsolutePath);

    /// <inheritdoc />
    public StorageSize Size()
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = DaemonClient.Call(
            (client, options) => client.GetFileMetadata(new GetFileMetadataRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.GetFileMetadata));
        return response.Size is null
            ? new StorageSize(0, "0 B")
            : DaemonModelFactory.ToStorageSize(response.Size);
    }

    /// <inheritdoc />
    public FileMetadata Metadata => _cachedMetadata ??= LoadMetadata(includeAnalysis: false);

    /// <inheritdoc />
    public FileMetadata RefreshMetadata(bool includeAnalysis = false)
    {
        EnsureNotDisposed();
        var info = LoadMetadata(includeAnalysis);
        if (includeAnalysis)
        {
            _cachedMetadataWithAnalysis = info;
            _cachedMetadata = info with { Analysis = null };
        }
        else
        {
            _cachedMetadata = info;
        }

        return info;
    }

    /// <inheritdoc />
    public AnalysisReport Analyze()
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = DaemonClient.Call(
            (client, options) => client.AnalyzePath(new AnalyzePathRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.AnalyzePath));
        return DaemonModelFactory.ToAnalysisReport(response, path);
    }

    /// <inheritdoc />
    public byte[] ReadBytes()
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = DaemonClient.Call(
            (client, options) => client.OpenReadHandle(new OpenReadHandleRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.OpenReadHandle));

        using var safeHandle = OpenReadHandle(response);
        using var source = new FileStream(safeHandle, FileAccess.Read, StreamBufferSize);
        using var buffer = new MemoryStream(checked((int)response.Size));
        source.CopyTo(buffer, StreamBufferSize);
        return buffer.ToArray();
    }

    /// <inheritdoc />
    public async Task<byte[]> ReadBytesAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.OpenReadHandleAsync(new OpenReadHandleRequest { Path = path }, options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.OpenReadHandle)).ConfigureAwait(false);

        using var safeHandle = OpenReadHandle(response);
        // DuplicateHandle / SCM_RIGHTS targets are opened without FILE_FLAG_OVERLAPPED — sync FileStream.
        using var source = new FileStream(safeHandle, FileAccess.Read, StreamBufferSize, isAsync: false);
        using var result = new MemoryStream(checked((int)response.Size));
        await CopyWithProgressAsync(source, result, response.Size, progress, cancellationToken).ConfigureAwait(false);
        return result.ToArray();
    }

    /// <inheritdoc />
    public string ReadText()
    {
        EnsureNotDisposed();
        return System.Text.Encoding.UTF8.GetString(ReadBytes());
    }

    /// <inheritdoc />
    public async Task<string> ReadTextAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var bytes = await ReadBytesAsync(progress, cancellationToken).ConfigureAwait(false);
        return System.Text.Encoding.UTF8.GetString(bytes);
    }

    /// <inheritdoc />
    public void Write(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true) =>
        WriteAsync(content, progress, overwrite, createParentDirectories).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task WriteAsync(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(content);
        using var stream = new MemoryStream(content, writable: false);
        await WriteAsync(stream, progress, overwrite, createParentDirectories, cancellationToken).ConfigureAwait(false);
    }

    /// <inheritdoc />
    public void WriteText(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true) =>
        Write(System.Text.Encoding.UTF8.GetBytes(text), progress, overwrite, createParentDirectories);

    /// <inheritdoc />
    public Task WriteTextAsync(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default) =>
        WriteAsync(System.Text.Encoding.UTF8.GetBytes(text), progress, overwrite, createParentDirectories, cancellationToken);

    /// <inheritdoc />
    public async Task WriteAsync(Stream stream, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        ArgumentNullException.ThrowIfNull(stream);

        var path = AbsolutePath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.OpenWriteHandleAsync(
                new OpenWriteHandleRequest
                {
                    Path = path,
                    Overwrite = overwrite,
                    CreateParentDirectories = createParentDirectories,
                },
                options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.OpenWriteHandle)).ConfigureAwait(false);

        using var safeHandle = OpenWriteHandle(response);
        // DuplicateHandle / SCM_RIGHTS targets are opened without FILE_FLAG_OVERLAPPED — sync FileStream.
        using var destination = new FileStream(safeHandle, FileAccess.Write, StreamBufferSize, isAsync: false);
        var total = stream.CanSeek ? (ulong?)stream.Length : null;
        await CopyWithProgressAsync(stream, destination, total, progress, cancellationToken).ConfigureAwait(false);
        await destination.FlushAsync(cancellationToken).ConfigureAwait(false);
        InvalidateCaches();
    }

    /// <inheritdoc />
    public IStorageFile Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false) =>
        CopyAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task<IStorageFile> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var source = AbsolutePath;
        var newPath = await DaemonClient.ConsumeCopyProgressAsync(
            (client, options) => client.CopyFile(new CopyFileRequest { Source = source, Destination = destination, Overwrite = overwrite }, options),
            progress,
            source,
            nameof(DharaSd.DharaSdClient.CopyFile),
            cancellationToken).ConfigureAwait(false);
        return new StorageFile(newPath);
    }

    /// <inheritdoc />
    public void Move(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false) =>
        MoveAsync(destination, progress, overwrite).GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task MoveAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var source = AbsolutePath;
        var response = await DaemonClient.CallAsync(
            (client, options) => client.MovePathAsync(new MovePathRequest { Source = source, Destination = destination, Overwrite = overwrite }, options),
            cancellationToken,
            source,
            nameof(DharaSd.DharaSdClient.MovePath)).ConfigureAwait(false);

        UpdatePathFromDestination(destination, response.Path);
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
    public void Delete() => DeleteAsync().GetAwaiter().GetResult();

    /// <inheritdoc />
    public async Task DeleteAsync(CancellationToken cancellationToken = default)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        await DaemonClient.CallAsync(
            (client, options) => client.DeletePathAsync(new DeletePathRequest { Path = path, Recursive = false }, options),
            cancellationToken,
            path,
            nameof(DharaSd.DharaSdClient.DeletePath)).ConfigureAwait(false);
        InvalidateCaches();
    }

    /// <inheritdoc />
    protected override void InvalidateCaches()
    {
        _cachedMetadata = null;
        _cachedMetadataWithAnalysis = null;
    }

    private FileMetadata LoadMetadata(bool includeAnalysis)
    {
        EnsureNotDisposed();
        var path = AbsolutePath;
        var response = DaemonClient.Call(
            (client, options) => client.GetFileMetadata(
                new GetFileMetadataRequest
                {
                    Path = path,
                    IncludeAnalysis = includeAnalysis,
                },
                options),
            path,
            nameof(DharaSd.DharaSdClient.GetFileMetadata));
        AnalysisReport? analysis = null;
        if (includeAnalysis && response.Analysis is not null)
        {
            analysis = DaemonModelFactory.ToAnalysisReport(response.Analysis, path);
        }

        return DaemonModelFactory.ToFileMetadata(response, analysis);
    }

    private static SafeFileHandle OpenReadHandle(OpenReadHandleResponse response) =>
        OperatingSystem.IsWindows()
            ? new SafeFileHandle((nint)response.Handle, ownsHandle: true)
            : DharaRuntime.ReceiveDataPlaneHandle();

    private static SafeFileHandle OpenWriteHandle(OpenWriteHandleResponse response) =>
        OperatingSystem.IsWindows()
            ? new SafeFileHandle((nint)response.Handle, ownsHandle: true)
            : DharaRuntime.ReceiveDataPlaneHandle();

    private static async Task CopyWithProgressAsync(
        Stream source,
        Stream destination,
        ulong? totalBytes,
        IProgress<StorageProgress>? progress,
        CancellationToken cancellationToken)
    {
        var buffer = new byte[StreamBufferSize];
        ulong transferred = 0;
        var started = DateTime.UtcNow;

        while (true)
        {
            var read = await source.ReadAsync(buffer.AsMemory(), cancellationToken).ConfigureAwait(false);
            if (read == 0)
            {
                break;
            }

            await destination.WriteAsync(buffer.AsMemory(0, read), cancellationToken).ConfigureAwait(false);
            transferred += (ulong)read;
            progress?.Report(new StorageProgress(totalBytes, transferred, ComputeRate(started, transferred)));
        }
    }

    private static double ComputeRate(DateTime startedUtc, ulong bytesTransferred)
    {
        var elapsedSeconds = (DateTime.UtcNow - startedUtc).TotalSeconds;
        return elapsedSeconds > 0 ? bytesTransferred / elapsedSeconds : 0;
    }
}
