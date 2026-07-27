using System.Diagnostics;

namespace Dhara.Storage.BenchPilot;

internal static class Timing
{
    public static ScenarioResult Measure(
        string name,
        string baseline,
        int iterations,
        Action action,
        long? payloadBytes = null,
        string? notes = null)
    {
        // Warmup
        action();

        var samples = new double[iterations];
        var allocatedBefore = GC.GetTotalAllocatedBytes(precise: true);
        var wall = Stopwatch.StartNew();
        for (var i = 0; i < iterations; i++)
        {
            var sw = Stopwatch.StartNew();
            action();
            sw.Stop();
            samples[i] = sw.Elapsed.TotalMilliseconds;
        }

        wall.Stop();
        var allocatedAfter = GC.GetTotalAllocatedBytes(precise: true);
        Array.Sort(samples);

        double? throughput = null;
        if (payloadBytes is > 0)
        {
            var seconds = wall.Elapsed.TotalSeconds;
            throughput = seconds <= 0 ? null : (payloadBytes.Value * iterations) / (1024.0 * 1024.0) / seconds;
        }

        return new ScenarioResult
        {
            Name = name,
            Baseline = baseline,
            Iterations = iterations,
            ElapsedMs = wall.Elapsed.TotalMilliseconds,
            P50Ms = Percentile(samples, 0.50),
            P95Ms = Percentile(samples, 0.95),
            ThroughputMBps = throughput,
            AllocatedBytes = allocatedAfter - allocatedBefore,
            Notes = notes,
        };
    }

    public static async Task<ScenarioResult> MeasureAsync(
        string name,
        string baseline,
        int iterations,
        Func<Task> action,
        long? payloadBytes = null,
        string? notes = null)
    {
        await action().ConfigureAwait(false);

        var samples = new double[iterations];
        var allocatedBefore = GC.GetTotalAllocatedBytes(precise: true);
        var wall = Stopwatch.StartNew();
        for (var i = 0; i < iterations; i++)
        {
            var sw = Stopwatch.StartNew();
            await action().ConfigureAwait(false);
            sw.Stop();
            samples[i] = sw.Elapsed.TotalMilliseconds;
        }

        wall.Stop();
        var allocatedAfter = GC.GetTotalAllocatedBytes(precise: true);
        Array.Sort(samples);

        double? throughput = null;
        if (payloadBytes is > 0)
        {
            var seconds = wall.Elapsed.TotalSeconds;
            throughput = seconds <= 0 ? null : (payloadBytes.Value * iterations) / (1024.0 * 1024.0) / seconds;
        }

        return new ScenarioResult
        {
            Name = name,
            Baseline = baseline,
            Iterations = iterations,
            ElapsedMs = wall.Elapsed.TotalMilliseconds,
            P50Ms = Percentile(samples, 0.50),
            P95Ms = Percentile(samples, 0.95),
            ThroughputMBps = throughput,
            AllocatedBytes = allocatedAfter - allocatedBefore,
            Notes = notes,
        };
    }

    private static double Percentile(double[] sorted, double p)
    {
        if (sorted.Length == 0)
        {
            return 0;
        }

        var index = (int)Math.Clamp(Math.Ceiling(p * sorted.Length) - 1, 0, sorted.Length - 1);
        return sorted[index];
    }
}
