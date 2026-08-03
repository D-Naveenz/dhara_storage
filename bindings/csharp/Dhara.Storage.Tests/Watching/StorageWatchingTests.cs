using Dhara.Storage.Models.Watching;
using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Watching;

public sealed class StorageWatchingTests
{
    [Fact]
    public async Task StartWatching_RaisesChangedEvent_WhenDirectoryChanges()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var temp = new TemporaryDirectory();
        var directory = DharaStorage.Directory(temp.Root);
        var completion = new TaskCompletionSource<StorageChangedEventArgs>(TaskCreationOptions.RunContinuationsAsynchronously);

        directory.Changed += (_, args) =>
        {
            if (args.Path.EndsWith("watched.txt", StringComparison.OrdinalIgnoreCase))
            {
                completion.TrySetResult(args);
            }
        };

        directory.StartWatching(new StorageWatchOptions
        {
            DebounceWindow = TimeSpan.FromMilliseconds(100),
            ReceiveTimeout = TimeSpan.FromMilliseconds(100),
            Recursive = true,
        });

        try
        {
            await System.IO.File.WriteAllTextAsync(temp.PathFor("watched.txt"), "watch me", cancellationToken);
            var change = await completion.Task.WaitAsync(TimeSpan.FromSeconds(10), cancellationToken);

            Assert.Contains(change.ChangeType, new[] { StorageChangeType.Created, StorageChangeType.Modified });
        }
        finally
        {
            directory.StopWatching();
        }
    }
}
