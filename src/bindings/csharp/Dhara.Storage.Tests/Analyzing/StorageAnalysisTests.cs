using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Analyzing;

public sealed class StorageAnalysisTests
{
    [Fact]
    public void AnalyzePath_ReturnsReport_ForExistingFile()
    {
        using var temp = new TemporaryDirectory();
        var filePath = temp.PathFor("notes.txt");
        File.WriteAllText(filePath, "hello from dhara storage");

        var report = DharaStorage.AnalyzePath(filePath);

        Assert.True(report.BytesScanned > 0);
        Assert.True(report.FileSize > 0);
    }
}
