using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;
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
    /// Gets cached file information, refreshing it on first use.
    /// </summary>
    /// <remarks>This property loads lightweight file information only. Use <see cref="RefreshInformation(bool)"/>
    /// with the method argument set to <see langword="true"/> when you also need content-analysis
    /// results in the returned <see cref="FileInformation"/> instance.</remarks>
    FileInformation Information { get; }

    /// <summary>
    /// Refreshes the cached file information.
    /// </summary>
    /// <param name="includeAnalysis"><see langword="true"/> to include content-analysis results in the refreshed snapshot;
    /// otherwise, <see langword="false"/> to refresh metadata only.</param>
    /// <returns>A new <see cref="FileInformation"/> snapshot for the current file path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the native runtime cannot read file information.</exception>
    FileInformation RefreshInformation(bool includeAnalysis = false);

    /// <summary>
    /// Runs content analysis for the current file.
    /// </summary>
    /// <returns>An <see cref="AnalysisReport"/> describing the strongest file-type matches for the current file.</returns>
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
    Task<byte[]> ReadBytesAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default);

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
    Task<string> ReadTextAsync(IProgress<StorageProgress>? progress = null, CancellationToken cancellationToken = default);

    /// <summary>
    /// Writes raw bytes to the current file.
    /// </summary>
    /// <param name="content">The full byte payload to write.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the buffered write path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing file; otherwise, <see langword="false"/> to fail if the file already exists.</param>
    /// <param name="createParentDirectories"><see langword="true"/> to create missing parent directories before writing.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be written.</exception>
    void Write(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true);

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
    Task WriteAsync(byte[] content, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Writes UTF-8 text to the current file.
    /// </summary>
    /// <param name="text">The UTF-8 text content to write.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the buffered write path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing file; otherwise, <see langword="false"/> to fail if the file already exists.</param>
    /// <param name="createParentDirectories"><see langword="true"/> to create missing parent directories before writing.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be written.</exception>
    void WriteText(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true);

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
    Task WriteTextAsync(string text, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

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
    Task WriteAsync(Stream stream, IProgress<StorageProgress>? progress = null, bool overwrite = true, bool createParentDirectories = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Copies the current file to the provided destination path.
    /// </summary>
    /// <param name="destination">The destination path for the copied file.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the asynchronous copy path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <returns>A new file wrapper pointing at the copied destination path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be copied.</exception>
    IStorageFile Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Copies the current file to the provided destination path asynchronously.
    /// </summary>
    /// <param name="destination">The destination path for the copied file.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native copy operation runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes with a new file wrapper pointing at the copied destination path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be copied.</exception>
    Task<IStorageFile> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Moves the current file to the provided destination path.
    /// </summary>
    /// <param name="destination">The destination path for the moved file.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the asynchronous move path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be moved.</exception>
    void Move(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Moves the current file to the provided destination path asynchronously.
    /// </summary>
    /// <param name="destination">The destination path for the moved file.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native move operation runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination file; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the file has been moved and the wrapper path has been updated.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the file cannot be moved.</exception>
    Task MoveAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

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
