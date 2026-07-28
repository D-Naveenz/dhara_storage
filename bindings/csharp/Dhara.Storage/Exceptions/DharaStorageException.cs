namespace Dhara.Storage.Exceptions;

/// <summary>
/// Represents a storage failure surfaced through the Dhara Storage daemon boundary.
/// </summary>
public sealed class DharaStorageException : Exception
{
    /// <summary>
    /// Initializes a new instance of the <see cref="DharaStorageException"/> class.
    /// </summary>
    /// <param name="message">Human-readable failure message from the daemon boundary.</param>
    /// <param name="code">Daemon / storage error code string.</param>
    /// <param name="path">Path associated with the failure, when available.</param>
    /// <param name="operation">Operation name, when available.</param>
    public DharaStorageException(string message, string code, string? path = null, string? operation = null)
        : base(message)
    {
        Code = code;
        PathValue = path;
        Operation = operation;
    }

    /// <summary>
    /// Gets the error code.
    /// </summary>
    public string Code { get; }

    /// <summary>
    /// Gets the path associated with the failure, when available.
    /// </summary>
    public string? PathValue { get; }

    /// <summary>
    /// Gets the underlying operation name, when available.
    /// </summary>
    public string? Operation { get; }
}
