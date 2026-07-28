using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;

namespace Dhara.Storage.Core;

/// <summary>
/// Central logger factory used by the managed Dhara Storage wrapper types.
/// </summary>
/// <remarks>The daemon (<c>dhara-sd</c>) logs to its own process output rather than across the
/// gRPC boundary, so this bridge only forwards managed wrapper log records; it does not bridge
/// native/daemon log records the way the retired in-process FFI logger did.</remarks>
internal static class DharaStorageLogBridge
{
    private static readonly object Gate = new();

    private static ILoggerFactory? _loggerFactory;

    /// <summary>Sets or clears the logger factory used to create loggers for wrapper log records.</summary>
    internal static void UseLoggerFactory(ILoggerFactory? loggerFactory)
    {
        lock (Gate)
        {
            if (ReferenceEquals(_loggerFactory, loggerFactory))
            {
                return;
            }

            _loggerFactory = loggerFactory;
            if (loggerFactory is not null)
            {
                LogManaged(LogLevel.Information, "Dhara.Storage.Logging", "Configured Dhara.Storage host logging.");
            }
        }
    }

    /// <summary>Logs a managed wrapper event through the configured logger factory, if any.</summary>
    internal static void LogManaged(
        LogLevel level,
        string category,
        string message,
        Exception? exception = null,
        IReadOnlyDictionary<string, object?>? fields = null)
    {
        var logger = CreateLogger(category);
        if (!logger.IsEnabled(level))
        {
            return;
        }

        using var scope = fields is null ? null : logger.BeginScope(fields);
        logger.Log(level, exception, "{Message}", message);
    }

    private static ILogger CreateLogger(string category) =>
        (_loggerFactory ?? NullLoggerFactory.Instance).CreateLogger(category);
}
