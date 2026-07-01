namespace Dhara.Storage.Exceptions;

/// <summary>
/// Represents a native storage failure surfaced through the Dhara Storage FFI boundary.
/// </summary>
public sealed class DharaStorageException : Exception
{
    /// <summary>
    /// Initializes a new instance of the <see cref="DharaStorageException"/> class.
    /// </summary>
    public DharaStorageException(string message, string code, string? path = null, string? operation = null)
        : base(message)
    {
        Code = code;
        PathValue = path;
        Operation = operation;
    }

    /// <summary>
    /// Gets the native error code.
    /// </summary>
    public string Code { get; }

    /// <summary>
    /// Gets the path associated with the native failure, when available.
    /// </summary>
    public string? PathValue { get; }

    /// <summary>
    /// Gets the underlying native operation name, when available.
    /// </summary>
    public string? Operation { get; }
}
