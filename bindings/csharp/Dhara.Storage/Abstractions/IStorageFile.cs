using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Metadata;
using Dhara.Storage.Models.Progress;

namespace Dhara.Storage.Abstractions;

/// <summary>
/// Represents a file wrapper that exposes sync and async operations over the native Dhara Storage runtime.
/// </summary>
/// <remarks>Implementations are path-based rather than handle-based. Operations always target the current
/// path represented by the instance, and methods that relocate the file update that path so the same wrapper
/// can continue to be used after a move or rename.</remarks>
public interface IStorageFile : IStorageItem
{
    /// <summary>
    /// Gets cached file metadata, refreshing it on first use.
    /// </summary>
    /// <remarks>Loads lightweight file metadata. After <see cref="Analyze"/>, subsequent
    /// metadata snapshots are enriched from the analysis cached on this wrapper. Use
    /// <see cref="RefreshMetadata(bool)"/> with <see langword="true"/> to force a fresh
    /// analysis as part of the metadata load.</remarks>
    FileMetadata Metadata { get; }

    /// <summary>
    /// Refreshes the cached file metadata.
    /// </summary>
    /// <param name="includeAnalysis"><see langword="true"/> to run content analysis now and include
    /// the report in the snapshot; otherwise, <see langword="false"/> to refresh metadata only
    /// (still enriched from a prior <see cref="Analyze"/> when one is cached).</param>
    /// <returns>A new <see cref="FileMetadata"/> snapshot for the current file path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the native runtime cannot read file metadata.</exception>
    FileMetadata RefreshMetadata(bool includeAnalysis = false);

    /// <summary>
    /// Measures the current file size on demand.
    /// </summary>
    /// <returns>A <see cref="StorageSize"/> snapshot for the current file path.</returns>
    /// <remarks>Size is measured by the daemon rather than cached as part of <see cref="Metadata"/>, since
    /// paths and size live on the storage wrapper, not on a metadata snapshot.</remarks>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the native runtime cannot read the file size.</exception>
    StorageSize Size();

    /// <summary>
    /// Runs content analysis for the current file and caches the report on this wrapper.
    /// </summary>
    /// <returns>An <see cref="AnalysisReport"/> describing the strongest file-type matches for the current file.</returns>
    /// <remarks>Subsequent <see cref="Metadata"/> / <see cref="RefreshMetadata"/> calls enrich type and
    /// extension from this cached report.</remarks>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be analyzed by the native runtime.</exception>
    AnalysisReport Analyze();

    /// <summary>
    /// Reads the current file into memory as raw bytes.
    /// </summary>
    /// <returns>A newly allocated byte array containing the full file contents.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be opened or read.</exception>
    byte[] ReadBytes();

    /// <summary>
    /// Reads the current file into memory asynchronously, optionally reporting progress.
    /// </summary>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native operation runs.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes with the full file contents as a byte array.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be opened or read.</exception>
    Task<byte[]> ReadBytesAsync(IProgress<StorageProcessEvent>? progress = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Reads the current file as UTF-8 text.
    /// </summary>
    /// <returns>The full file contents decoded as UTF-8 text.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be read.</exception>
    string ReadText();

    /// <summary>
    /// Reads the current file as UTF-8 text asynchronously, optionally reporting progress.
    /// </summary>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native operation runs.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes with the full file contents decoded as UTF-8 text.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be read or does not contain valid UTF-8 text.</exception>
    Task<string> ReadTextAsync(IProgress<StorageProcessEvent>? progress = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Writes raw bytes to the current file.
    /// </summary>
    /// <param name="content">The full byte payload to write.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the buffered write path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing file; otherwise, <see langword="false"/> to fail if the file already exists.</param>
    /// <param name="createParentDirectories"><see langword="true"/> to create missing parent directories before writing.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be written.</exception>
    void Write(byte[] content, IProgress<StorageProcessEvent>? progress = null, bool overwrite = true, bool createParentDirectories = true);

    /// <summary>
    /// Writes raw bytes to the current file asynchronously.
    /// </summary>
    /// <param name="content">The full byte payload to write.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native operation runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing file; otherwise, <see langword="false"/> to fail if the file already exists.</param>
    /// <param name="createParentDirectories"><see langword="true"/> to create missing parent directories before writing.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the write finishes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be written.</exception>
    Task WriteAsync(byte[] content, IProgress<StorageProcessEvent>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Writes UTF-8 text to the current file.
    /// </summary>
    /// <param name="text">The UTF-8 text content to write.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the buffered write path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing file; otherwise, <see langword="false"/> to fail if the file already exists.</param>
    /// <param name="createParentDirectories"><see langword="true"/> to create missing parent directories before writing.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be written.</exception>
    void WriteText(string text, IProgress<StorageProcessEvent>? progress = null, bool overwrite = true, bool createParentDirectories = true);

    /// <summary>
    /// Writes UTF-8 text to the current file asynchronously.
    /// </summary>
    /// <param name="text">The UTF-8 text content to write.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native operation runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing file; otherwise, <see langword="false"/> to fail if the file already exists.</param>
    /// <param name="createParentDirectories"><see langword="true"/> to create missing parent directories before writing.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the write finishes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be written.</exception>
    Task WriteTextAsync(string text, IProgress<StorageProcessEvent>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Streams content into the current file asynchronously.
    /// </summary>
    /// <param name="stream">The source stream whose contents should be copied into the destination file.</param>
    /// <param name="progress">An optional progress sink that receives best-effort transfer updates as the stream is copied.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing file; otherwise, <see langword="false"/> to fail if the file already exists.</param>
    /// <param name="createParentDirectories"><see langword="true"/> to create missing parent directories before writing.</param>
    /// <param name="cancellationToken">A token used to request cancellation while the managed stream copy is in progress.</param>
    /// <returns>A task that completes when the stream has been copied and the native write session has been finalized.</returns>
    /// <exception cref="ArgumentNullException">Thrown when <paramref name="stream"/> is <see langword="null"/>.</exception>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the native write session cannot be created or finalized.</exception>
    Task WriteAsync(Stream stream, IProgress<StorageProcessEvent>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Copies the current file to the provided destination path.
    /// </summary>
    /// <param name="destination">The destination path for the copied file.</param>
    /// <param name="progress">An optional progress sink that receives transfer events while the copy runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be copied.</exception>
    void Copy(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false);

    /// <summary>
    /// Starts copying the current file immediately and returns a running <see cref="StorageProcess"/>.
    /// </summary>
    /// <param name="destination">The destination path for the copied file.</param>
    /// <param name="progress">An optional progress sink that receives transfer events while the copy runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A <see cref="StorageProcess"/> that can be awaited for the destination path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be copied.</exception>
    StorageProcess CopyAsync(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Moves the current file to the provided destination path.
    /// </summary>
    /// <param name="destination">The destination path for the moved file.</param>
    /// <param name="progress">An optional progress sink that receives transfer events when the move path reports them.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be moved.</exception>
    void Move(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false);

    /// <summary>
    /// Starts moving the current file immediately and returns a running <see cref="StorageProcess"/>.
    /// </summary>
    /// <param name="destination">The destination path for the moved file.</param>
    /// <param name="progress">An optional progress sink that receives transfer events when the move path reports them.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A <see cref="StorageProcess"/> that can be awaited for the destination path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be moved.</exception>
    StorageProcess MoveAsync(string destination, IProgress<StorageProcessEvent>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Renames the current file within its existing parent directory.
    /// </summary>
    /// <param name="newName">The new file name to apply within the current parent directory.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be renamed.</exception>
    void Rename(string newName);

    /// <summary>
    /// Renames the current file asynchronously.
    /// </summary>
    /// <param name="newName">The new file name to apply within the current parent directory.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the rename finishes and the wrapper path has been updated.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be renamed.</exception>
    Task RenameAsync(string newName, CancellationToken cancellationToken = default);

    /// <summary>
    /// Deletes the current file.
    /// </summary>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be deleted.</exception>
    void Delete();

    /// <summary>
    /// Deletes the current file asynchronously.
    /// </summary>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the delete finishes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be deleted.</exception>
    Task DeleteAsync(CancellationToken cancellationToken = default);
}
