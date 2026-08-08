using System.Globalization;
using System.Text.Json.Serialization;
using BenchmarkDotNet.Reports;
using BenchmarkDotNet.Running;

namespace Dhara.Storage.Benchmarks.Reporting;

/// <summary>
/// Builds comparison rows and raw stats from a BenchmarkDotNet <see cref="Summary"/>.
/// </summary>
internal static class EvidenceModel
{
    public static IReadOnlyList<ComparisonRow> BuildComparisons(Summary summary)
    {
        var byMethod = IndexByMethod(summary);
        var rows = new List<ComparisonRow>(ScenarioCatalog.Rung1Pairs.Count);
        foreach (var pair in ScenarioCatalog.Rung1Pairs)
        {
            byMethod.TryGetValue(pair.BaselineMethod, out var baseline);
            byMethod.TryGetValue(pair.CandidateMethod, out var candidate);
            if (baseline is null || candidate is null)
            {
                rows.Add(ComparisonRow.Missing(pair));
                continue;
            }

            var ratio = baseline.MeanNs > 0 ? candidate.MeanNs / baseline.MeanNs : double.NaN;
            long allocatedDelta = 0;
            if (baseline.AllocatedBytes is { } bAlloc && candidate.AllocatedBytes is { } cAlloc)
            {
                allocatedDelta = cAlloc - bAlloc;
            }

            var verdict = Evaluate(pair.MaxMeanOverhead, ratio);
            rows.Add(new ComparisonRow(
                pair,
                baseline,
                candidate,
                ratio,
                allocatedDelta,
                verdict));
        }

        return rows;
    }

    public static IReadOnlyList<MethodStats> BuildCandidateOnly(Summary summary)
    {
        var byMethod = IndexByMethod(summary);
        var rows = new List<MethodStats>();
        foreach (var name in ScenarioCatalog.Rung1CandidateOnlyMethods)
        {
            if (byMethod.TryGetValue(name, out var stats))
            {
                rows.Add(stats);
            }
        }

        return rows;
    }

    public static IReadOnlyList<MethodStats> BuildAllMethods(Summary summary)
    {
        var list = new List<MethodStats>(summary.Reports.Length);
        foreach (var report in summary.Reports)
        {
            if (TryFromReport(report, out var stats))
            {
                list.Add(stats);
            }
        }

        return list;
    }

    public static ScenarioVerdict Evaluate(double? maxMeanOverhead, double ratio)
    {
        if (double.IsNaN(ratio) || double.IsInfinity(ratio))
        {
            return ScenarioVerdict.Missing;
        }

        if (maxMeanOverhead is null)
        {
            return ScenarioVerdict.Informational;
        }

        var overhead = ratio - 1.0;
        if (overhead <= maxMeanOverhead.Value)
        {
            return ScenarioVerdict.Pass;
        }

        // Soft band: up to 1.5× the allowed overhead → Watch; beyond → Fail.
        if (overhead <= maxMeanOverhead.Value * 1.5)
        {
            return ScenarioVerdict.Watch;
        }

        return ScenarioVerdict.Fail;
    }

    public static Dictionary<string, MethodStats> IndexByMethod(Summary summary)
    {
        var map = new Dictionary<string, MethodStats>(StringComparer.Ordinal);
        foreach (var report in summary.Reports)
        {
            if (TryFromReport(report, out var stats))
            {
                map[stats.Method] = stats;
            }
        }

        return map;
    }

    public static bool TryFromReport(BenchmarkReport report, out MethodStats stats)
    {
        var method = report.BenchmarkCase.Descriptor.WorkloadMethod.Name;
        var title = report.BenchmarkCase.Descriptor.WorkloadMethodDisplayInfo;
        var statistics = report.ResultStatistics;
        if (statistics is null)
        {
            stats = default!;
            return false;
        }

        // MemoryDiagnoser path: bytes allocated per operation when available.
        var allocated = report.GcStats.GetBytesAllocatedPerOperation(report.BenchmarkCase);

        stats = new MethodStats(
            method,
            title,
            statistics.Mean,
            statistics.StandardError,
            statistics.StandardDeviation,
            allocated,
            report.GcStats.Gen0Collections,
            report.GcStats.Gen1Collections,
            report.GcStats.Gen2Collections);
        return true;
    }

    /// <summary>Reads host fields from BenchmarkDotNet 0.15+ <see cref="BenchmarkDotNet.Environments.HostEnvironmentInfo"/>.</summary>
    public static HostSnapshot ReadHost(BenchmarkDotNet.Environments.HostEnvironmentInfo host)
    {
        var lines = host.ToFormattedString().ToList();
        var os = lines.ElementAtOrDefault(0) ?? "—";
        var cpu = lines.ElementAtOrDefault(1) ?? "—";
        // First line includes BDN version + OS brand; strip the BDN prefix when present.
        const string caption = "BenchmarkDotNet v";
        if (os.StartsWith(caption, StringComparison.Ordinal))
        {
            var comma = os.IndexOf(',', StringComparison.Ordinal);
            if (comma > 0 && comma + 1 < os.Length)
            {
                os = os[(comma + 1)..].Trim();
            }
        }

        string sdk;
        try
        {
            sdk = host.DotNetSdkVersion.Value ?? "—";
        }
        catch
        {
            sdk = "—";
        }

        return new HostSnapshot(
            Os: ShortenOs(os),
            Processor: cpu,
            Cores: "—",
            Runtime: ShortenRuntime(host.RuntimeVersion ?? "—"),
            DotNetSdk: sdk,
            BenchmarkDotNet: host.BenchmarkDotNetVersion ?? "—",
            Architecture: host.Architecture ?? "—",
            Configuration: host.Configuration ?? "—");
    }

    /// <summary>
    /// Trims Windows marketing suffixes (e.g. <c>/25H2/2025Update/…</c>) that break card layout.
    /// </summary>
    public static string ShortenOs(string os)
    {
        if (string.IsNullOrWhiteSpace(os) || os == "—")
        {
            return "—";
        }

        // "Windows 11 (10.0.26200.8973/25H2/…)" → "Windows 11 (10.0.26200.8973)"
        var slash = os.IndexOf('/');
        if (slash > 0 && os.Contains('(', StringComparison.Ordinal))
        {
            return os[..slash].TrimEnd() + ")";
        }

        return os;
    }

    /// <summary>Drops redundant build metadata from runtime strings when present.</summary>
    public static string ShortenRuntime(string runtime)
    {
        if (string.IsNullOrWhiteSpace(runtime) || runtime == "—")
        {
            return "—";
        }

        // ".NET 10.0.10 (10.0.10, 10.0.1026.32716)" → ".NET 10.0.10"
        var open = runtime.IndexOf(" (", StringComparison.Ordinal);
        return open > 0 ? runtime[..open] : runtime;
    }

    /// <summary>Compact job line for the host card (warmup / iterations / runtime).</summary>
    public static string FormatJobSummary(BenchmarkDotNet.Running.BenchmarkCase benchmarkCase)
    {
        var job = benchmarkCase.Job;
        var runtime = job.Environment.Runtime?.Name ?? ".NET";
        var warmups = job.Run.WarmupCount;
        var iterations = job.Run.IterationCount;
        return $"{runtime}, warmup {warmups}, iterations {iterations}";
    }

    public static string FormatTime(double nanoseconds) =>
        nanoseconds switch
        {
            < 1_000 => $"{nanoseconds:F2} ns",
            < 1_000_000 => $"{nanoseconds / 1_000:F2} μs",
            < 1_000_000_000 => $"{nanoseconds / 1_000_000:F2} ms",
            _ => $"{nanoseconds / 1_000_000_000:F3} s",
        };

    public static string FormatBytes(long? bytes)
    {
        if (bytes is null)
        {
            return "—";
        }

        var value = (double)bytes.Value;
        return value switch
        {
            < 1024 => $"{bytes.Value} B",
            < 1024 * 1024 => $"{value / 1024:F2} KB",
            _ => $"{value / (1024 * 1024):F2} MB",
        };
    }

    public static string FormatRatio(double ratio) =>
        double.IsNaN(ratio) ? "—" : ratio.ToString("0.00×", CultureInfo.InvariantCulture);

    public static string FormatSignedBytes(long delta)
    {
        var sign = delta > 0 ? "+" : string.Empty;
        return sign + FormatBytes(delta);
    }

    /// <summary>
    /// Session-stamped base name aligned with BDN log files (e.g. <c>…BindingBenchmarks-20260807-145440</c>).
    /// </summary>
    public static string SessionFileStem(Summary summary)
    {
        var stem = summary.Title;
        if (string.IsNullOrWhiteSpace(stem))
        {
            stem = "benchmark-" + DateTime.UtcNow.ToString("yyyyMMdd-HHmmss");
        }

        foreach (var c in Path.GetInvalidFileNameChars())
        {
            stem = stem.Replace(c, '_');
        }

        return stem;
    }

    public static string SessionHtmlPath(Summary summary) =>
        Path.Combine(summary.ResultsDirectoryPath, SessionFileStem(summary) + "-report.html");

    public static string SessionJsonPath(Summary summary) =>
        Path.Combine(summary.ResultsDirectoryPath, SessionFileStem(summary) + "-results.json");
}

/// <summary>Host environment strings for reports.</summary>
internal sealed record HostSnapshot(
    string Os,
    string Processor,
    string Cores,
    string Runtime,
    string DotNetSdk,
    string BenchmarkDotNet,
    string Architecture,
    string Configuration);

/// <summary>Per-method stats extracted from a BenchmarkDotNet report.</summary>
internal sealed record MethodStats(
    string Method,
    string Title,
    double MeanNs,
    double ErrorNs,
    double StdDevNs,
    long? AllocatedBytes,
    double Gen0,
    double Gen1,
    double Gen2);

/// <summary>One paired comparison row for the human report.</summary>
internal sealed record ComparisonRow(
    ScenarioPair Pair,
    MethodStats? Baseline,
    MethodStats? Candidate,
    double Ratio,
    long AllocatedDelta,
    ScenarioVerdict Verdict)
{
    public static ComparisonRow Missing(ScenarioPair pair) =>
        new(pair, null, null, double.NaN, 0, ScenarioVerdict.Missing);
}

/// <summary>Machine-oriented payload written to <c>results.json</c>.</summary>
internal sealed class ResultsDocument
{
    public string Schema { get; init; } = "dhara.benchmarks.results/v1";
    public string Title { get; init; } = "";
    public string GeneratedUtc { get; init; } = "";
    public string Rung { get; init; } = "daemon-vs-ffi";
    public HostInfoDto Host { get; init; } = new();
    public List<MethodResultDto> Benchmarks { get; init; } = [];
    public List<ComparisonDto> Comparisons { get; init; } = [];
}

internal sealed class HostInfoDto
{
    public string Os { get; init; } = "";
    public string Processor { get; init; } = "";
    public string Runtime { get; init; } = "";
    public string DotNetSdk { get; init; } = "";
    public string BenchmarkDotNet { get; init; } = "";
    public string Architecture { get; init; } = "";
}

internal sealed class MethodResultDto
{
    public string Method { get; init; } = "";
    public string Title { get; init; } = "";
    public double MeanNs { get; init; }
    public double ErrorNs { get; init; }
    public double StdDevNs { get; init; }
    public long? AllocatedBytes { get; init; }
    public double Gen0 { get; init; }
    public double Gen1 { get; init; }
    public double Gen2 { get; init; }
}

internal sealed class ComparisonDto
{
    public string Id { get; init; } = "";
    public string DisplayName { get; init; } = "";
    public string BaselineMethod { get; init; } = "";
    public string CandidateMethod { get; init; } = "";
    public double? Ratio { get; init; }
    public long? AllocatedDeltaBytes { get; init; }
    public double? MaxMeanOverhead { get; init; }

    [JsonConverter(typeof(JsonStringEnumConverter))]
    public ScenarioVerdict Verdict { get; init; }
}
