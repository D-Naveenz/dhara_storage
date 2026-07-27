namespace Dhara.Storage.BenchPilot;

internal static class ScenarioTimeouts
{
    public static async Task<ScenarioResult> RunAsync(
        string name,
        string baseline,
        int iterations,
        Func<Task> action,
        TimeSpan timeout,
        long? payloadBytes = null,
        string? notes = null)
    {
        try
        {
            using var cts = new CancellationTokenSource(timeout);
            var task = Timing.MeasureAsync(name, baseline, iterations, async () =>
            {
                cts.Token.ThrowIfCancellationRequested();
                await action().ConfigureAwait(false);
            }, payloadBytes, notes);
            return await task.WaitAsync(cts.Token).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is OperationCanceledException or TimeoutException)
        {
            return new ScenarioResult
            {
                Name = name,
                Baseline = baseline,
                Iterations = iterations,
                Notes = $"timed out after {timeout.TotalSeconds:F0}s",
            };
        }
    }
}
