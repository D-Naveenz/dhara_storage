using System.Text.Json;
using System.Text.Json.Serialization;
using BenchmarkDotNet.Exporters;
using BenchmarkDotNet.Loggers;
using BenchmarkDotNet.Reports;

namespace Dhara.Storage.Benchmarks.Reporting;

/// <summary>
/// Writes machine-oriented raw stats under BDN's results directory,
/// session-stamped like the BDN log (e.g. <c>…-20260807-145440-results.json</c>).
/// </summary>
internal sealed class ResultsJsonExporter : IExporter
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    public string Name => nameof(ResultsJsonExporter);

    public void ExportToLog(Summary summary, ILogger logger)
    {
    }

    public IEnumerable<string> ExportToFiles(Summary summary, ILogger consoleLogger)
    {
        Directory.CreateDirectory(summary.ResultsDirectoryPath);
        var path = EvidenceModel.SessionJsonPath(summary);
        var document = BuildDocument(summary);
        File.WriteAllText(path, JsonSerializer.Serialize(document, JsonOptions));
        consoleLogger.WriteLineInfo($"Evidence JSON: {path}");
        yield return path;
    }

    private static ResultsDocument BuildDocument(Summary summary)
    {
        var host = EvidenceModel.ReadHost(summary.HostEnvironmentInfo);
        var comparisons = EvidenceModel.BuildComparisons(summary);
        var methods = EvidenceModel.BuildAllMethods(summary);

        return new ResultsDocument
        {
            Title = summary.Title,
            GeneratedUtc = DateTime.UtcNow.ToString("o"),
            Host = new HostInfoDto
            {
                Os = host.Os,
                Processor = host.Processor,
                Runtime = host.Runtime,
                DotNetSdk = host.DotNetSdk,
                BenchmarkDotNet = host.BenchmarkDotNet,
                Architecture = host.Architecture,
            },
            Benchmarks = methods.Select(static m => new MethodResultDto
            {
                Method = m.Method,
                Title = m.Title,
                MeanNs = m.MeanNs,
                ErrorNs = m.ErrorNs,
                StdDevNs = m.StdDevNs,
                AllocatedBytes = m.AllocatedBytes,
                Gen0 = m.Gen0,
                Gen1 = m.Gen1,
                Gen2 = m.Gen2,
            }).ToList(),
            Comparisons = comparisons.Select(static row => new ComparisonDto
            {
                Id = row.Pair.Id,
                DisplayName = row.Pair.DisplayName,
                BaselineMethod = row.Pair.BaselineMethod,
                CandidateMethod = row.Pair.CandidateMethod,
                Ratio = double.IsNaN(row.Ratio) ? null : row.Ratio,
                AllocatedDeltaBytes = row.Baseline is null || row.Candidate is null ? null : row.AllocatedDelta,
                MaxMeanOverhead = row.Pair.MaxMeanOverhead,
                Verdict = row.Verdict,
            }).ToList(),
        };
    }
}
