using BenchmarkDotNet.Columns;
using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Reports;
using BenchmarkDotNet.Running;
using Dhara.Storage.Benchmarks.Reporting;

namespace Dhara.Storage.Benchmarks;

/// <summary>
/// Entry point: BenchmarkDotNet harness with custom HTML + JSON evidence reports.
/// </summary>
internal static class Program
{
    public static int Main(string[] args)
    {
        if (!OperatingSystem.IsWindows())
        {
            Console.Error.WriteLine("Binding harness is Windows-only in this milestone.");
            return 2;
        }

        var smoke = args.Any(static a => a is "--smoke" or "--smoke-bdn");
        var bdnArgs = args
            .Where(static a => a is not ("--smoke" or "--smoke-bdn"))
            .ToArray();

        var artifacts = Path.Combine(DaemonHost.FindRepoRoot(), "target", "benchmarks", "bdn");
        Directory.CreateDirectory(artifacts);

        // Minimum-viable config avoids DefaultConfig CSV/HTML/Markdown sprawl.
        // Plots (RPlotExporter) are not enabled; add here later if needed.
        var config = ManualConfig
            .CreateMinimumViable()
            .WithArtifactsPath(artifacts)
            .HideColumns(Column.Median, Column.Ratio, Column.RatioSD)
            .AddExporter(new EvidenceHtmlExporter())
            .AddExporter(new ResultsJsonExporter())
            .WithOptions(ConfigOptions.DisableOptimizationsValidator);

        Summary summary;
        if (smoke)
        {
            Console.WriteLine("Running BindingSmokeBenchmarks via BenchmarkDotNet.");
            summary = BenchmarkRunner.Run<BindingSmokeBenchmarks>(config, bdnArgs);
        }
        else
        {
            Console.WriteLine("Running BindingBenchmarks via BenchmarkDotNet.");
            Console.WriteLine("Tip: --smoke for a short suite; --filter *Copy* to select methods.");
            summary = BenchmarkRunner.Run<BindingBenchmarks>(config, bdnArgs);
        }

        Console.WriteLine($"Artifacts: {artifacts}");
        Console.WriteLine($"  Human:   {EvidenceModel.SessionHtmlPath(summary)}");
        Console.WriteLine($"  Machine: {EvidenceModel.SessionJsonPath(summary)}");
        return 0;
    }
}
