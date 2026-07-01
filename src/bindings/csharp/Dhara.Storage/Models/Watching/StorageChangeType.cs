namespace Dhara.Storage.Models.Watching;

/// <summary>
/// Describes the high-level type of a debounced directory change event.
/// </summary>
public enum StorageChangeType
{
    /// <summary>
    /// A file-system entry was created.
    /// </summary>
    Created,

    /// <summary>
    /// A file-system entry was deleted.
    /// </summary>
    Deleted,

    /// <summary>
    /// A file-system entry was modified in place.
    /// </summary>
    Modified,

    /// <summary>
    /// A file-system entry was moved or renamed.
    /// </summary>
    Relocated,
}
