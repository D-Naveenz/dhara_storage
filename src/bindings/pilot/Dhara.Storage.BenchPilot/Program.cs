using Dhara.Storage.Pilot.V1;

namespace Dhara.Storage.BenchPilot;

internal static class Program
{
    public static async Task<int> Main(string[] args)
    {
        if (!OperatingSystem.IsWindows())
        {
            Console.Error.WriteLine("Binding pilot harness is Windows-only in this milestone.");
            return 2;
        }

        var quick = !args.Any(static a => a is "--full");
        var smoke = args.Any(static a => a is "--smoke");
        Console.WriteLine(smoke
            ? "Running smoke binding-pilot checks."
            : quick
                ? "Running quick binding-pilot matrix (pass --full for complete sizes)."
                : "Running full binding-pilot matrix.");

        await using var host = await DaemonHost.StartAsync().ConfigureAwait(false);
        Console.WriteLine($"Daemon ready on pipe '{host.PipeName}' (pid {host.DaemonProcessId}).");

        var report = smoke
            ? await ScenarioRunner.RunSmokeAsync(host).ConfigureAwait(false)
            : await ScenarioRunner.RunAsync(host, quick).ConfigureAwait(false);
        ReportWriter.Write(report);
        Console.WriteLine();
        Console.WriteLine(report.Recommendation);
        return 0;
    }
}
