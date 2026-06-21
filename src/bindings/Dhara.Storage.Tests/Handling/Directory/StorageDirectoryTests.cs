using Dhara.Storage.Models.Progress;
using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Handling.Directory;

public sealed class StorageDirectoryTests
{
    [Fact]
    public void GetEntries_EnumeratesRecursiveTree()
    {
        using var temp = new TemporaryDirectory();
        System.IO.Directory.CreateDirectory(temp.PathFor("docs", "nested"));
        System.IO.File.WriteAllText(temp.PathFor("docs", "a.txt"), "A");
        System.IO.File.WriteAllText(temp.PathFor("docs", "nested", "b.txt"), "B");
        var directory = DharaStorage.Directory(temp.PathFor("docs"));

        var entries = directory.GetEntries(recursive: true);

        Assert.True(entries.Count >= 3);
        Assert.Contains(entries, entry => entry.Path.EndsWith("a.txt", StringComparison.OrdinalIgnoreCase));
        Assert.Contains(entries, entry => entry.Path.EndsWith("nested", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public async Task CopyAsync_CopiesDirectoryTree()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var temp = new TemporaryDirectory();
        System.IO.Directory.CreateDirectory(temp.PathFor("source", "nested"));
        System.IO.File.WriteAllText(temp.PathFor("source", "nested", "file.txt"), "payload");
        var directory = DharaStorage.Directory(temp.PathFor("source"));
        var progressValues = new List<StorageProgress>();
        var progress = new SynchronousProgress<StorageProgress>(progressValues.Add);

        var copy = await directory.CopyAsync(temp.PathFor("copy"), progress, overwrite: false, cancellationToken);

        Assert.True(System.IO.Directory.Exists(copy.FullPath));
        Assert.True(System.IO.File.Exists(Path.Combine(copy.FullPath, "nested", "file.txt")));
        Assert.NotEmpty(progressValues);
    }
}
