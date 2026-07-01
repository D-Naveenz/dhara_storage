using System.Threading;
using Microsoft.Extensions.Logging;

namespace Dhara.Storage.Tests.TestSupport;

internal sealed class CapturingLoggerFactory(LogLevel minimumLevel = LogLevel.Trace) : ILoggerFactory
{
    private readonly object _gate = new();
    private readonly List<CapturedLogEntry> _entries = [];
    private readonly AsyncLocal<ScopeNode?> _currentScope = new();

    public IReadOnlyList<CapturedLogEntry> Entries
    {
        get
        {
            lock (_gate)
            {
                return _entries.ToArray();
            }
        }
    }

    public void AddProvider(ILoggerProvider provider)
    {
    }

    public ILogger CreateLogger(string categoryName) => new CapturingLogger(this, categoryName, minimumLevel);

    public void Dispose()
    {
    }

    private IDisposable PushScope(object state)
    {
        var previous = _currentScope.Value;
        _currentScope.Value = new ScopeNode(state, previous);
        return new ScopeHandle(this, previous);
    }

    private void AddEntry(LogLevel level, string category, EventId eventId, string message, Exception? exception, object? state)
    {
        var fields = new Dictionary<string, object?>(StringComparer.OrdinalIgnoreCase);
        CopyStructuredValues(state, fields);

        for (var current = _currentScope.Value; current is not null; current = current.Previous)
        {
            CopyStructuredValues(current.State, fields);
        }

        lock (_gate)
        {
            _entries.Add(new CapturedLogEntry(level, category, eventId, message, exception, fields));
        }
    }

    private static void CopyStructuredValues(object? value, IDictionary<string, object?> destination)
    {
        if (value is null)
        {
            return;
        }

        if (value is IEnumerable<KeyValuePair<string, object?>> structuredNullable)
        {
            foreach (var pair in structuredNullable)
            {
                destination[pair.Key] = pair.Value;
            }

            return;
        }

        if (value is IEnumerable<KeyValuePair<string, object>> structured)
        {
            foreach (var pair in structured)
            {
                destination[pair.Key] = pair.Value;
            }

            return;
        }

        destination[value.GetType().Name] = value;
    }

    internal sealed record CapturedLogEntry(
        LogLevel Level,
        string Category,
        EventId EventId,
        string Message,
        Exception? Exception,
        IReadOnlyDictionary<string, object?> Fields);

    private sealed record ScopeNode(object State, ScopeNode? Previous);

    private sealed class ScopeHandle(CapturingLoggerFactory factory, ScopeNode? previous) : IDisposable
    {
        public void Dispose() => factory._currentScope.Value = previous;
    }

    private sealed class CapturingLogger(
        CapturingLoggerFactory factory,
        string categoryName,
        LogLevel minimumLevel) : ILogger
    {
        public IDisposable BeginScope<TState>(TState state) where TState : notnull => factory.PushScope(state);

        public bool IsEnabled(LogLevel logLevel) => logLevel >= minimumLevel;

        public void Log<TState>(
            LogLevel logLevel,
            EventId eventId,
            TState state,
            Exception? exception,
            Func<TState, Exception?, string> formatter)
        {
            if (!IsEnabled(logLevel))
            {
                return;
            }

            factory.AddEntry(logLevel, categoryName, eventId, formatter(state, exception), exception, state);
        }
    }
}
