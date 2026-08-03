using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;

namespace Dhara.Storage.Extensions.Hosting;

/// <summary>
/// Dependency injection registration for the <c>dhara-sd</c> sidecar lifetime.
/// </summary>
public static class DharaStorageServiceCollectionExtensions
{
    /// <summary>
    /// Registers a hosted service that starts <c>dhara-sd</c> when the host starts and stops it
    /// when the host shuts down.
    /// </summary>
    /// <param name="services">The service collection to register against.</param>
    /// <param name="configure">Optional delegate used to configure sidecar startup options.</param>
    /// <returns>The same <see cref="IServiceCollection"/> instance, to allow call chaining.</returns>
    public static IServiceCollection AddDharaStorage(this IServiceCollection services, Action<DharaStorageHostingOptions>? configure = null)
    {
        ArgumentNullException.ThrowIfNull(services);

        var options = new DharaStorageHostingOptions();
        configure?.Invoke(options);

        services.AddSingleton(options);
        services.AddHostedService<DharaStorageHostedService>();
        return services;
    }
}
