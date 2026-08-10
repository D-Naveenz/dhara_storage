using Microsoft.Extensions.Logging;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Logging;

public sealed class StorageLoggingTests
{
    [Fact]
    public async Task UseLoggerFactory_DemotesDaemonClientWrapperLogs()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var temp = new TemporaryDirectory();
        var path = temp.PathFor("logging.bin");
        await File.WriteAllBytesAsync(path, Enumerable.Repeat((byte)7, 128 * 1024).ToArray(), cancellationToken);

        using var loggerFactory = new CapturingLoggerFactory(LogLevel.Information);

        DharaStorage.UseLoggerFactory(loggerFactory);
        try
        {
            var file = DharaStorage.File(path);
            var progress = new SynchronousProgress<StorageProcessEvent>(_ => { });
            await file.ReadBytesAsync(progress, cancellationToken);
        }
        finally
        {
            DharaStorage.UseLoggerFactory(null);
        }

        // Wrapper RPC chatter is demoted to Trace; daemon tracing uses StreamLogs + Rust targets.
        Assert.DoesNotContain(loggerFactory.Entries, entry =>
            entry.Category == "Dhara.Storage.DaemonClient" &&
            entry.Level >= LogLevel.Information);
    }
}
