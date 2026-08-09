using Dhara.Storage.Abstractions;

namespace Dhara.Storage.Core;

/// <summary>
/// Provides shared path/state behavior for public storage wrappers.
/// </summary>
public abstract class StorageItemBase : IStorageItem
{
    private readonly object _stateGate = new();
    private string _absolutePath;
    private string? _relativePath;
    private bool _disposed;

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageItemBase"/> class.
    /// </summary>
    protected StorageItemBase(string path)
    {
        _absolutePath = System.IO.Path.GetFullPath(path);
        _relativePath = System.IO.Path.IsPathRooted(path) ? null : path;
    }

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageItemBase"/> class from a daemon-resolved
    /// path, preserving <paramref name="destination"/> as the tracked relative path when it was not rooted.
    /// </summary>
    /// <param name="resolvedPath">The resolved path reported by the native runtime (for example after a copy).</param>
    /// <param name="destination">The destination originally supplied by the caller.</param>
    protected StorageItemBase(string resolvedPath, string destination)
    {
        _absolutePath = System.IO.Path.GetFullPath(resolvedPath);
        _relativePath = System.IO.Path.IsPathRooted(destination) ? null : destination;
    }

    /// <inheritdoc />
    public string AbsolutePath
    {
        get
        {
            lock (_stateGate)
            {
                return _absolutePath;
            }
        }
    }

    /// <inheritdoc />
    public string? RelativePath
    {
        get
        {
            lock (_stateGate)
            {
                return _relativePath;
            }
        }
    }

    /// <inheritdoc />
    public string Name => System.IO.Path.GetFileName(AbsolutePath.TrimEnd(System.IO.Path.DirectorySeparatorChar, System.IO.Path.AltDirectorySeparatorChar));

    /// <inheritdoc />
    public abstract bool Exists { get; }

    /// <summary>
    /// Throws when the instance has already been disposed.
    /// </summary>
    protected void EnsureNotDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    /// <summary>
    /// Updates the current path after an operation that resolves to a definite absolute location
    /// (for example rename), clearing any tracked relative-path input.
    /// </summary>
    protected void UpdatePathAbsolute(string resolvedPath)
    {
        lock (_stateGate)
        {
            _absolutePath = System.IO.Path.GetFullPath(resolvedPath);
            _relativePath = null;
        }

        InvalidateCaches();
    }

    /// <summary>
    /// Updates the current path after a move or copy to <paramref name="destination"/>, preserving
    /// <paramref name="destination"/> as the tracked relative path when it was not rooted.
    /// </summary>
    protected void UpdatePathFromDestination(string destination, string resolvedPath)
    {
        lock (_stateGate)
        {
            _absolutePath = System.IO.Path.GetFullPath(resolvedPath);
            _relativePath = System.IO.Path.IsPathRooted(destination) ? null : destination;
        }

        InvalidateCaches();
    }

    /// <summary>
    /// Refreshes the resolved absolute path after an in-place operation (for example directory creation)
    /// without changing the tracked relative-path input.
    /// </summary>
    protected void RefreshResolvedPath(string resolvedPath)
    {
        lock (_stateGate)
        {
            _absolutePath = System.IO.Path.GetFullPath(resolvedPath);
        }

        InvalidateCaches();
    }

    /// <summary>
    /// Clears any cached state associated with the current path.
    /// </summary>
    protected abstract void InvalidateCaches();

    /// <inheritdoc />
    public virtual void Dispose()
    {
        _disposed = true;
        GC.SuppressFinalize(this);
    }
}
