namespace Dhara.Storage.Extensions.Hosting;

/// <summary>
/// Configures how <see cref="DharaStorageHostedService"/> starts the <c>dhara-sd</c> sidecar.
/// </summary>
public sealed class DharaStorageHostingOptions
{
    /// <summary>
    /// Gets or sets the named pipe suffix to request from the daemon; a unique name is
    /// generated when left <see langword="null"/>.
    /// </summary>
    public string? PipeName { get; set; }

    /// <summary>
    /// Gets or sets an explicit path to the <c>dhara-sd</c> executable, overriding automatic
    /// resolution relative to the application base directory and repository layout.
    /// </summary>
    public string? DaemonExePath { get; set; }
}
