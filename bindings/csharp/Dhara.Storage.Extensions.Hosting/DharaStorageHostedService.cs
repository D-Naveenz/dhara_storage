using Dhara.Storage.Runtime;
using Microsoft.Extensions.Hosting;

namespace Dhara.Storage.Extensions.Hosting;

/// <summary>
/// Starts the <c>dhara-sd</c> sidecar with the host and stops it during host shutdown.
/// </summary>
internal sealed class DharaStorageHostedService(DharaStorageHostingOptions options) : IHostedService
{
    /// <inheritdoc />
    public Task StartAsync(CancellationToken cancellationToken) =>
        DharaRuntime.StartAsync(options.PipeName, options.DaemonExePath, cancellationToken);

    /// <inheritdoc />
    public async Task StopAsync(CancellationToken cancellationToken)
    {
        if (DharaRuntime.Current is { } runtime)
        {
            await runtime.DisposeAsync().ConfigureAwait(false);
        }
    }
}
