using Dhara.Storage.Abstractions;

namespace Dhara.Storage.Core;

/// <summary>
/// Provides shared path/state behavior for public storage wrappers.
/// </summary>
public abstract class StorageItemBase : IStorageItem
{
    private readonly object _stateGate = new();
    private string _fullPath;
    private bool _disposed;

    /// <summary>
    /// Initializes a new instance of the <see cref="StorageItemBase"/> class.
    /// </summary>
    protected StorageItemBase(string path)
    {
        _fullPath = System.IO.Path.GetFullPath(path);
    }

    /// <inheritdoc />
    public string FullPath
    {
        get
        {
            lock (_stateGate)
            {
                return _fullPath;
            }
        }
    }

    /// <inheritdoc />
    public string Name => System.IO.Path.GetFileName(FullPath.TrimEnd(System.IO.Path.DirectorySeparatorChar, System.IO.Path.AltDirectorySeparatorChar));

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
    /// Updates the current absolute path after a successful move or rename.
    /// </summary>
    protected void UpdatePath(string path)
    {
        lock (_stateGate)
        {
            _fullPath = System.IO.Path.GetFullPath(path);
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
