using Microsoft.Extensions.Logging;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Logging;

public sealed class StorageLoggingTests
{
    [Fact]
    public async Task UseLoggerFactory_ForwardsManagedAndNativeLogs()
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

        var entries = loggerFactory.Entries;
        Assert.Contains(entries, entry =>
            entry.Category == "Dhara.Storage.NativeOperationHandle" &&
            entry.Level == LogLevel.Information &&
            entry.Message.Contains("completed", StringComparison.OrdinalIgnoreCase));

        var nativeEntry = entries.FirstOrDefault(entry =>
            entry.Category == "dhara_storage::operations::file" &&
            entry.Fields.TryGetValue("nativeTarget", out var target) &&
            string.Equals(target?.ToString(), "dhara_storage::operations::file", StringComparison.Ordinal));
        Assert.NotNull(nativeEntry);
        var confirmedNativeEntry = nativeEntry!;
        Assert.Equal("dhara_storage::operations::file", confirmedNativeEntry.Fields["nativeTarget"]);
        Assert.Contains(confirmedNativeEntry.Fields.Keys, key => key.StartsWith("native.", StringComparison.Ordinal));
    }
}
