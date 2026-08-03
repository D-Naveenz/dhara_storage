using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Exporters.Json;
using BenchmarkDotNet.Running;

namespace Dhara.Storage.Benchmarks;

/// <summary>
/// Entry point: BenchmarkDotNet only (plus daemon/fixture helpers).
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

        var artifacts = Path.Combine(DaemonHost.FindRepoRoot(), "target", "bench-pilot", "bdn");
        Directory.CreateDirectory(artifacts);

        var config = ManualConfig
            .Create(DefaultConfig.Instance)
            .WithArtifactsPath(artifacts)
            .AddExporter(JsonExporter.Full)
            .WithOptions(ConfigOptions.DisableOptimizationsValidator);

        if (smoke)
        {
            Console.WriteLine("Running BindingSmokeBenchmarks via BenchmarkDotNet.");
            _ = BenchmarkRunner.Run<BindingSmokeBenchmarks>(config, bdnArgs);
        }
        else
        {
            Console.WriteLine("Running BindingBenchmarks via BenchmarkDotNet.");
            Console.WriteLine("Tip: --smoke for a short suite; --filter *Copy* to select methods.");
            _ = BenchmarkRunner.Run<BindingBenchmarks>(config, bdnArgs);
        }

        Console.WriteLine($"Artifacts: {artifacts}");
        return 0;
    }
}
