using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Information;

public sealed class ShellIconTests
{
    [Fact]
    public void GetFileInformation_IncludeIcon_ReturnsShellMetadataWhenAvailable()
    {
        using var temp = new TemporaryDirectory();
        var path = temp.PathFor("sample.txt");
        System.IO.File.WriteAllText(path, "icon probe");

        var info = DharaStorage.GetFileInformation(path, includeAnalysis: false, includeIcon: true, iconSize: 32);

        // Shell metadata depends on the host desktop environment; under CI (Xvfb on Linux) or
        // Windows locally an icon is usually available for a plain text file.
        if (info.Icon is not null)
        {
            Assert.True(info.Icon.Width > 0);
            Assert.True(info.Icon.Height > 0);
            Assert.NotEmpty(info.Icon.RgbaPixels);
        }

        if (info.ShellDetails is not null)
        {
            Assert.False(string.IsNullOrWhiteSpace(info.ShellDetails.DisplayName)
                && string.IsNullOrWhiteSpace(info.ShellDetails.TypeName));
        }
    }
}
