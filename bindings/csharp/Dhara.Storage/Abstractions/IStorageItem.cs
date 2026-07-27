using System.IO;

namespace Dhara.Storage.Abstractions;

/// <summary>
/// Represents a path-based storage item surfaced by the Dhara Storage .NET bindings.
/// </summary>
/// <remarks>The .NET wrapper keeps only lightweight path state on the managed side. Metadata, analysis,
/// mutation, and watching work is delegated to the native Dhara Storage runtime as needed, which keeps this
/// abstraction inexpensive to construct and safe to cache for short-lived workflows.</remarks>
public interface IStorageItem : IDisposable
{
    /// <summary>
    /// Gets the absolute file-system path represented by this instance.
    /// </summary>
    /// <remarks>The path is normalized to a full path when the wrapper is created or when an operation
    /// changes the item's location. The value may continue to point to a missing path after delete or move
    /// operations until the instance is refreshed by the API that changed it.</remarks>
    string FullPath { get; }

    /// <summary>
    /// Gets the last path segment for the current item.
    /// </summary>
    /// <remarks>For root paths, this value reflects the platform-specific root display segment returned by
    /// <see cref="Path.GetFileName(string)"/> fallback behavior in the wrapper.</remarks>
    string Name { get; }

    /// <summary>
    /// Gets a value indicating whether the represented path currently exists.
    /// </summary>
    /// <remarks>This property performs a lightweight filesystem existence check and does not force a full
    /// metadata refresh from the native runtime.</remarks>
    bool Exists { get; }
}
