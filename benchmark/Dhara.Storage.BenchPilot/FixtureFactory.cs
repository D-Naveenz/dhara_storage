using System.Security.Cryptography;

namespace Dhara.Storage.BenchPilot;

/// <summary>
/// Creates reproducible fixture trees under <c>target/bench-pilot/fixtures</c>.
/// </summary>
internal static class FixtureFactory
{
    public static string Root { get; } = Path.Combine(DaemonHost.FindRepoRoot(), "target", "bench-pilot", "fixtures");

    public static string EnsureSizedFile(string name, long sizeBytes)
    {
        Directory.CreateDirectory(Root);
        var path = Path.Combine(Root, name);
        if (File.Exists(path) && new FileInfo(path).Length == sizeBytes)
        {
            return path;
        }

        using var stream = File.Create(path);
        var buffer = new byte[Math.Min(sizeBytes, 1024 * 1024)];
        RandomNumberGenerator.Fill(buffer);
        long remaining = sizeBytes;
        while (remaining > 0)
        {
            var chunk = (int)Math.Min(remaining, buffer.Length);
            stream.Write(buffer, 0, chunk);
            remaining -= chunk;
        }

        return path;
    }

    public static string EnsureListingDirectory(string name, int fileCount)
    {
        var dir = Path.Combine(Root, name);
        Directory.CreateDirectory(dir);
        for (var i = 0; i < fileCount; i++)
        {
            var path = Path.Combine(dir, $"entry-{i:D5}.bin");
            if (!File.Exists(path))
            {
                File.WriteAllBytes(path, [(byte)(i % 256)]);
            }
        }

        return dir;
    }

    public static string? CoreFixture(string fileName)
    {
        var path = Path.Combine(
            DaemonHost.FindRepoRoot(),
            "core",
            "dhara_storage",
            "tests",
            "fixtures",
            fileName);
        return File.Exists(path) ? path : null;
    }
}
