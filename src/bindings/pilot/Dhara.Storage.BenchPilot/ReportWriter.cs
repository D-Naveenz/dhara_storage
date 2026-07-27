using System.Text.Json;
using System.Text.Json.Serialization;

namespace Dhara.Storage.BenchPilot;

internal sealed class BenchReport
{
    public string GeneratedAtUtc { get; set; } = DateTime.UtcNow.ToString("O");

    public string Machine { get; set; } = Environment.MachineName;

    public string Os { get; set; } = Environment.OSVersion.ToString();

    public List<ScenarioResult> Scenarios { get; set; } = [];

    public string? Recommendation { get; set; }
}

internal sealed class ScenarioResult
{
    public required string Name { get; set; }

    public required string Baseline { get; set; }

    public int Iterations { get; set; }

    public double ElapsedMs { get; set; }

    public double P50Ms { get; set; }

    public double P95Ms { get; set; }

    public double? ThroughputMBps { get; set; }

    public long? AllocatedBytes { get; set; }

    public string? Notes { get; set; }
}

internal static class ReportWriter
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public static string OutputDirectory { get; } =
        Path.Combine(DaemonHost.FindRepoRoot(), "target", "bench-pilot");

    public static void Write(BenchReport report)
    {
        Directory.CreateDirectory(OutputDirectory);
        var stamp = DateTime.UtcNow.ToString("yyyyMMdd-HHmmss");
        var jsonPath = Path.Combine(OutputDirectory, $"results-{stamp}.json");
        var mdPath = Path.Combine(OutputDirectory, $"results-{stamp}.md");
        var latestJson = Path.Combine(OutputDirectory, "results-latest.json");
        var latestMd = Path.Combine(OutputDirectory, "results-latest.md");

        var json = JsonSerializer.Serialize(report, JsonOptions);
        File.WriteAllText(jsonPath, json);
        File.WriteAllText(latestJson, json);

        var md = ToMarkdown(report);
        File.WriteAllText(mdPath, md);
        File.WriteAllText(latestMd, md);

        Console.WriteLine($"Wrote {jsonPath}");
        Console.WriteLine($"Wrote {mdPath}");
    }

    private static string ToMarkdown(BenchReport report)
    {
        var lines = new List<string>
        {
            "# Binding pilot results",
            string.Empty,
            $"- Generated: {report.GeneratedAtUtc}",
            $"- Machine: {report.Machine}",
            $"- OS: {report.Os}",
            string.Empty,
            "| Scenario | Baseline | Iter | Elapsed ms | p50 ms | p95 ms | MB/s | Alloc bytes | Notes |",
            "|----------|----------|------|------------|--------|--------|------|-------------|-------|",
        };

        foreach (var row in report.Scenarios)
        {
            lines.Add(
                $"| {row.Name} | {row.Baseline} | {row.Iterations} | {row.ElapsedMs:F2} | {row.P50Ms:F2} | {row.P95Ms:F2} | {row.ThroughputMBps?.ToString("F2") ?? "-"} | {row.AllocatedBytes?.ToString() ?? "-"} | {row.Notes ?? ""} |");
        }

        if (!string.IsNullOrWhiteSpace(report.Recommendation))
        {
            lines.Add(string.Empty);
            lines.Add("## Recommendation");
            lines.Add(string.Empty);
            lines.Add(report.Recommendation);
        }

        return string.Join(Environment.NewLine, lines);
    }
}
