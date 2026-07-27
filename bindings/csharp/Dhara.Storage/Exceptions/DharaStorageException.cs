namespace Dhara.Storage.Exceptions;

/// <summary>
/// Represents a native storage failure surfaced through the Dhara Storage FFI boundary.
/// </summary>
public sealed class DharaStorageException : Exception
{
    /// <summary>
    /// Initializes a new instance of the <see cref="DharaStorageException"/> class.
    /// </summary>
    /// <param name="message">Human-readable failure message from the native boundary.</param>
    /// <param name="code">Native error code string.</param>
    /// <param name="path">Path associated with the failure, when available.</param>
    /// <param name="operation">Native operation name, when available.</param>
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
