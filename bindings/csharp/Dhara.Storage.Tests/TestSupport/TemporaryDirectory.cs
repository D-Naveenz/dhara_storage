namespace Dhara.Storage.Tests.TestSupport;

internal sealed class TemporaryDirectory : IDisposable
{
    internal TemporaryDirectory()
    {
        Root = Path.Combine(Path.GetTempPath(), "dhara-storage-tests", Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Root);
    }

    internal string Root { get; }

    internal string PathFor(params string[] segments)
    {
        var path = Root;
        foreach (var segment in segments)
        {
            path = Path.Combine(path, segment);
        }

        return path;
    }

    public void Dispose()
    {
        if (Directory.Exists(Root))
        {
            Directory.Delete(Root, recursive: true);
        }
    }
}
