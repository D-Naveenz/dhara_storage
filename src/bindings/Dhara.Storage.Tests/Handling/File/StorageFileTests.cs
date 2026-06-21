using Dhara.Storage.Models.Progress;
using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Handling.File;

public sealed class StorageFileTests
{
    [Fact]
    public void WriteText_ThenReadText_RoundTripsContent()
    {
        using var temp = new TemporaryDirectory();
        var file = DharaStorage.File(temp.PathFor("roundtrip.txt"));

        file.WriteText("hello world");

        Assert.True(file.Exists);
        Assert.Equal("hello world", file.ReadText());
        Assert.True(file.Information.Size > 0);
    }

    [Fact]
    public async Task CopyAsync_ReportsProgress_AndCreatesDestination()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var temp = new TemporaryDirectory();
        var sourcePath = temp.PathFor("source.bin");
        await System.IO.File.WriteAllBytesAsync(sourcePath, Enumerable.Repeat((byte)42, 512 * 1024).ToArray(), cancellationToken);
        var file = DharaStorage.File(sourcePath);
        var reported = new List<StorageProgress>();
        var progress = new SynchronousProgress<StorageProgress>(reported.Add);

        var copy = await file.CopyAsync(temp.PathFor("copy.bin"), progress, overwrite: false, cancellationToken);

        Assert.True(System.IO.File.Exists(copy.FullPath));
        Assert.NotEmpty(reported);
        Assert.True(reported[^1].BytesTransferred > 0);
    }

    [Fact]
    public async Task WriteAsync_Stream_UsesSessionAndPersistsContent()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var temp = new TemporaryDirectory();
        var file = DharaStorage.File(temp.PathFor("stream.txt"));
        await using var stream = new MemoryStream(System.Text.Encoding.UTF8.GetBytes("streamed text"));

        await file.WriteAsync(stream, cancellationToken: cancellationToken);

        Assert.Equal("streamed text", file.ReadText());
    }
}
