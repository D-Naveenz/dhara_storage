using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Logging.Abstractions;
using Dhara.Storage.Interop.Native;

namespace Dhara.Storage.Core;

internal static class DharaStorageLogBridge
{
    private static readonly object Gate = new();

    private static ILoggerFactory? _loggerFactory;
    private static bool _nativeBridgeRegistered;

    internal static void UseLoggerFactory(ILoggerFactory? loggerFactory)
    {
        lock (Gate)
        {
            if (ReferenceEquals(_loggerFactory, loggerFactory))
            {
                return;
            }

            if (loggerFactory is null)
            {
                if (_nativeBridgeRegistered)
                {
                    NativeLogging.UnregisterLogger();
                    _nativeBridgeRegistered = false;
                }

                _loggerFactory = null;
                return;
            }

            _loggerFactory = loggerFactory;
            NativeLogging.RegisterLogger();
            _nativeBridgeRegistered = true;
            LogManaged(LogLevel.Information, "Dhara.Storage.Logging", "Configured Dhara.Storage host logging.");
        }
    }

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

    internal static void LogNative(NativeLogRecordDto record)
    {
        var logger = CreateLogger(record.Target);
        var level = record.ToLogLevel();
        if (!logger.IsEnabled(level))
        {
            return;
        }

        var scope = new Dictionary<string, object?>(StringComparer.OrdinalIgnoreCase)
        {
            ["nativeTimestampUnixMs"] = record.TimestampUnixMs,
            ["nativeLevel"] = record.Level,
            ["nativeTarget"] = record.Target,
        };

        if (!string.IsNullOrWhiteSpace(record.ModulePath))
        {
            scope["nativeModulePath"] = record.ModulePath;
        }

        if (!string.IsNullOrWhiteSpace(record.File))
        {
            scope["nativeFile"] = record.File;
        }

        if (record.Line.HasValue)
        {
            scope["nativeLine"] = record.Line.Value;
        }

        foreach (var field in record.Fields)
        {
            scope[$"native.{field.Key}"] = field.Value;
        }

        using var _ = logger.BeginScope(scope);
        logger.Log(level, "{Message}", record.Message);
    }

    private static ILogger CreateLogger(string category) =>
        (_loggerFactory ?? NullLoggerFactory.Instance).CreateLogger(category);
}
