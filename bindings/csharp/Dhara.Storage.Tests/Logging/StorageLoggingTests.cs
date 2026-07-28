using Microsoft.Extensions.Logging;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Logging;

public sealed class StorageLoggingTests
{
    [Fact]
    public async Task UseLoggerFactory_ForwardsManagedDaemonClientLogs()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var temp = new TemporaryDirectory();
        var path = temp.PathFor("logging.bin");
        await File.WriteAllBytesAsync(path, Enumerable.Repeat((byte)7, 128 * 1024).ToArray(), cancellationToken);

        using var loggerFactory = new CapturingLoggerFactory(LogLevel.Debug);

        DharaStorage.UseLoggerFactory(loggerFactory);
        try
        {
            var file = DharaStorage.File(path);
            var progress = new SynchronousProgress<StorageProgress>(_ => { });
            await file.ReadBytesAsync(progress, cancellationToken);
        }
        finally
        {
            DharaStorage.UseLoggerFactory(null);
        }

        // The dhara-sd daemon logs to its own process output rather than across the gRPC
        // boundary, so only managed wrapper log records (from DaemonClient) are observable here.
        var entries = loggerFactory.Entries;
        Assert.Contains(entries, entry =>
            entry.Category == "Dhara.Storage.DaemonClient" &&
            entry.Level == LogLevel.Debug &&
            entry.Message.Contains("OpenReadHandle", StringComparison.Ordinal));
        Assert.Contains(entries, entry =>
            entry.Category == "Dhara.Storage.DaemonClient" &&
            entry.Level == LogLevel.Information &&
            entry.Message.Contains("completed", StringComparison.OrdinalIgnoreCase));
    }
}
