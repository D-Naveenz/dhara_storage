using Dhara.Storage.Tests.TestSupport;

namespace Dhara.Storage.Tests.Information;

public sealed class ShellIconTests
{
    [Fact]
    public void GetFileInformation_IncludeIcon_IconNotSupportedOverDaemonTransport()
    {
        using var temp = new TemporaryDirectory();
        var path = temp.PathFor("sample.txt");
        System.IO.File.WriteAllText(path, "icon probe");

        var info = DharaStorage.GetFileInformation(path, includeAnalysis: false, includeIcon: true, iconSize: 32);

        // Shell icons required an in-process native call; the dhara-sd daemon transport does not
        // expose an equivalent RPC yet, so this always returns null today.
        Assert.Null(info.Icon);
        Assert.Null(info.ShellDetails);
    }
}
