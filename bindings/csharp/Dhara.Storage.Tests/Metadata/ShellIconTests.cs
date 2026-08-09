using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Metadata;

public sealed class ShellIconTests
{
    [Fact]
    public void GetFileMetadata_IncludeIcon_ReturnsShellMetadataWhenAvailable()
    {
        using var temp = new TemporaryDirectory();
        var path = temp.PathFor("sample.txt");
        System.IO.File.WriteAllText(path, "icon probe");

        var metadata = DharaStorage.GetFileMetadata(path, includeAnalysis: false, includeIcon: true, iconSize: 32);

        // Shell metadata depends on the host desktop environment; under CI (Xvfb on Linux) or
        // Windows locally an icon is usually available for a plain text file.
        if (metadata.Icon is not null)
        {
            Assert.True(metadata.Icon.Width > 0);
            Assert.True(metadata.Icon.Height > 0);
            Assert.True(metadata.Icon.RgbaPixels.Length > 0);
        }

        Assert.False(string.IsNullOrWhiteSpace(metadata.DisplayName)
            && string.IsNullOrWhiteSpace(metadata.FileType.Name));
    }
}
