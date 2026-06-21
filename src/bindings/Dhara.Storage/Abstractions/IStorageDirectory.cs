using Dhara.Storage.Models.Information;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Models.Watching;

namespace Dhara.Storage.Abstractions;

/// <summary>
/// Represents a directory wrapper that exposes enumeration, mutation, and watch capabilities over the native runtime.
/// </summary>
/// <remarks>Directory wrappers can be used both for immediate path-based operations and for longer-lived watch
/// sessions. Watching is explicit so callers can decide when background native resources should be allocated and
/// when debounced change notifications should stop.</remarks>
public interface IStorageDirectory : IStorageItem
{
    /// <summary>
    /// Gets cached directory information, refreshing it on first use.
    /// </summary>
    /// <remarks>This property loads lightweight metadata only. Use <see cref="RefreshInformation(bool)"/> with
    /// the method argument set to <see langword="true"/> when you need recursive counts and size totals.</remarks>
    DirectoryInformation Information { get; }

    /// <summary>
    /// Occurs when a watched directory emits a debounced change notification.
    /// </summary>
    /// <remarks>The event is raised only after <see cref="StartWatching(StorageWatchOptions?)"/> succeeds and
    /// continues until <see cref="StopWatching"/> or <see cref="IDisposable.Dispose"/> is called.</remarks>
    event EventHandler<StorageChangedEventArgs>? Changed;

    /// <summary>
    /// Gets a value indicating whether directory watching is active for this instance.
    /// </summary>
    /// <remarks>This property reflects whether the managed watch loop is currently running for this wrapper.</remarks>
    bool IsWatching { get; }

    /// <summary>
    /// Refreshes the cached directory information.
    /// </summary>
    /// <param name="includeSummary"><see langword="true"/> to include recursive size and entry counts; otherwise,
    /// <see langword="false"/> to refresh metadata only.</param>
    /// <returns>A new <see cref="DirectoryInformation"/> snapshot for the current directory path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the native runtime cannot read directory information.</exception>
    DirectoryInformation RefreshInformation(bool includeSummary = false);

    /// <summary>
    /// Enumerates child files.
    /// </summary>
    /// <param name="recursive"><see langword="true"/> to include files from nested directories; otherwise, <see langword="false"/> for the immediate directory only.</param>
    /// <returns>A snapshot list of file wrappers under the current directory.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be enumerated.</exception>
    IReadOnlyList<StorageFile> GetFiles(bool recursive = false);

    /// <summary>
    /// Enumerates child directories.
    /// </summary>
    /// <param name="recursive"><see langword="true"/> to include nested directories recursively; otherwise, <see langword="false"/> for the immediate directory only.</param>
    /// <returns>A snapshot list of directory wrappers under the current directory.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be enumerated.</exception>
    IReadOnlyList<StorageDirectory> GetDirectories(bool recursive = false);

    /// <summary>
    /// Enumerates child files and directories.
    /// </summary>
    /// <param name="recursive"><see langword="true"/> to include nested entries recursively; otherwise, <see langword="false"/> for the immediate directory only.</param>
    /// <returns>A snapshot list of entry models describing child files and directories.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be enumerated.</exception>
    IReadOnlyList<StorageEntry> GetEntries(bool recursive = false);

    /// <summary>
    /// Resolves a file relative to the current directory.
    /// </summary>
    /// <param name="relativePath">A relative path rooted at the current directory.</param>
    /// <returns>A file wrapper for the resolved path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    StorageFile GetFile(string relativePath);

    /// <summary>
    /// Resolves a directory relative to the current directory.
    /// </summary>
    /// <param name="relativePath">A relative path rooted at the current directory.</param>
    /// <returns>A directory wrapper for the resolved path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    StorageDirectory GetDirectory(string relativePath);

    /// <summary>
    /// Creates the current directory path.
    /// </summary>
    /// <returns>The current wrapper after the directory has been created.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be created.</exception>
    StorageDirectory Create();

    /// <summary>
    /// Creates the current directory path and any missing parents.
    /// </summary>
    /// <returns>The current wrapper after the directory tree has been created.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory tree cannot be created.</exception>
    StorageDirectory CreateAll();

    /// <summary>
    /// Creates the current directory path asynchronously.
    /// </summary>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes with the current wrapper once the directory has been created.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be created.</exception>
    Task<StorageDirectory> CreateAsync(CancellationToken cancellationToken = default);

    /// <summary>
    /// Creates the current directory path and any missing parents asynchronously.
    /// </summary>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes with the current wrapper once the directory tree has been created.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory tree cannot be created.</exception>
    Task<StorageDirectory> CreateAllAsync(CancellationToken cancellationToken = default);

    /// <summary>
    /// Copies the current directory tree to the provided destination path.
    /// </summary>
    /// <param name="destination">The destination root path for the copied directory tree.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the asynchronous copy path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination tree; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <returns>A new directory wrapper pointing at the copied destination path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory tree cannot be copied.</exception>
    IStorageDirectory Copy(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Copies the current directory tree to the provided destination path asynchronously.
    /// </summary>
    /// <param name="destination">The destination root path for the copied directory tree.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native copy operation runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination tree; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes with a new directory wrapper pointing at the copied destination path.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory tree cannot be copied.</exception>
    Task<IStorageDirectory> CopyAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Moves the current directory tree to the provided destination path.
    /// </summary>
    /// <param name="destination">The destination root path for the moved directory tree.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots when the asynchronous move path is used.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination tree; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory tree cannot be moved.</exception>
    void Move(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false);

    /// <summary>
    /// Moves the current directory tree to the provided destination path asynchronously.
    /// </summary>
    /// <param name="destination">The destination root path for the moved directory tree.</param>
    /// <param name="progress">An optional progress sink that receives transfer snapshots while the native move operation runs.</param>
    /// <param name="overwrite"><see langword="true"/> to replace an existing destination tree; otherwise, <see langword="false"/> to fail if the destination already exists.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the move finishes and the wrapper path has been updated.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory tree cannot be moved.</exception>
    Task MoveAsync(string destination, IProgress<StorageProgress>? progress = null, bool overwrite = false, CancellationToken cancellationToken = default);

    /// <summary>
    /// Renames the current directory within its existing parent directory.
    /// </summary>
    /// <param name="newName">The new directory name to apply within the current parent directory.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be renamed.</exception>
    void Rename(string newName);

    /// <summary>
    /// Renames the current directory asynchronously.
    /// </summary>
    /// <param name="newName">The new directory name to apply within the current parent directory.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the rename finishes and the wrapper path has been updated.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be renamed.</exception>
    Task RenameAsync(string newName, CancellationToken cancellationToken = default);

    /// <summary>
    /// Deletes the current directory.
    /// </summary>
    /// <param name="recursive"><see langword="true"/> to delete the directory tree recursively; otherwise, <see langword="false"/> to request a non-recursive delete.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be deleted.</exception>
    void Delete(bool recursive = true);

    /// <summary>
    /// Deletes the current directory asynchronously.
    /// </summary>
    /// <param name="recursive"><see langword="true"/> to delete the directory tree recursively; otherwise, <see langword="false"/> to request a non-recursive delete.</param>
    /// <param name="cancellationToken">A token used to request cooperative cancellation of the native operation.</param>
    /// <returns>A task that completes when the delete finishes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="OperationCanceledException">Thrown when <paramref name="cancellationToken"/> cancels the operation.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the directory cannot be deleted.</exception>
    Task DeleteAsync(bool recursive = true, CancellationToken cancellationToken = default);

    /// <summary>
    /// Starts explicit directory watching for this instance.
    /// </summary>
    /// <param name="options">Optional watch settings that control recursion, debounce timing, and receive behavior.</param>
    /// <exception cref="ObjectDisposedException">Thrown when the wrapper has already been disposed.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the native directory watcher cannot be created.</exception>
    void StartWatching(StorageWatchOptions? options = null);

    /// <summary>
    /// Stops directory watching for this instance.
    /// </summary>
    /// <remarks>Calling this method when no watch session is active is safe and has no effect.</remarks>
    void StopWatching();
}
