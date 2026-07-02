using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Information;

public sealed class ShellIconTests
{
    [Fact]
    public void GetFileInformation_IncludeIcon_ReturnsValidRgbaIcon()
    {
        if (!OperatingSystem.IsWindows() && !OperatingSystem.IsLinux() && !OperatingSystem.IsMacOS())
        {
            return;
        }

        using var temp = new TemporaryDirectory();
        var path = temp.PathFor("sample.txt");
        System.IO.File.WriteAllText(path, "icon probe");

        var info = DharaStorage.GetFileInformation(path, includeAnalysis: false, includeIcon: true, iconSize: 32);

        Assert.NotNull(info.Icon);
        Assert.True(info.Icon.Width > 0);
        Assert.True(info.Icon.Height > 0);
        Assert.True(info.Icon.IsValid);
        Assert.True(info.Icon.RgbaPixels.Length > 0);
    }
}
