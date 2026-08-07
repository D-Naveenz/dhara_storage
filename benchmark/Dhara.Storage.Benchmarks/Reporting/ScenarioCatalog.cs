namespace Dhara.Storage.Benchmarks.Reporting;

/// <summary>
/// Baseline/candidate pairing and decision thresholds for evidence reports.
/// </summary>
/// <remarks>
/// Method names are BenchmarkDotNet workload method names (not <c>Description</c> strings).
/// Thresholds for rung 1 match <c>docs/binding-benchmarks.md</c>. Future suites add rows with a
/// C# approximation baseline instead of FFI — exporters stay unchanged.
/// </remarks>
internal static class ScenarioCatalog
{
    /// <summary>Rung-1 paired scenarios (FFI baseline vs daemon candidate).</summary>
    public static IReadOnlyList<ScenarioPair> Rung1Pairs { get; } =
    [
        new("get-file-info", "GetFileInfo", "B1_GetFileInfo", "B2_GetFileInfo", MaxMeanOverhead: null),
        new("list-entries-100", "ListEntries / 100", "B1_ListEntries100", "B2_ListEntries100", MaxMeanOverhead: 0.15),
        new("analyze-path", "AnalyzePath", "B1_AnalyzePath", "B2_AnalyzePath", MaxMeanOverhead: 0.10),
        new("read-4k", "Read 4KB (handle dup)", "B1_Read4K", "B2_ReadHandleDup4K", MaxMeanOverhead: null),
        // Competitive with B1: allow 5% noise before Watch; above that Fail.
        new("read-1m", "Read 1MB (handle dup)", "B1_Read1M", "B2_ReadHandleDup1M", MaxMeanOverhead: 0.05),
        new("write-1m", "Write 1MB (handle dup)", "B1_Write1M", "B2_WriteHandleDup1M", MaxMeanOverhead: null),
        new("copy-1m", "Copy 1MB", "B1_Copy1M", "B2_Copy1M", MaxMeanOverhead: 0.10),
    ];

    /// <summary>Daemon-only methods with no FFI peer (still listed for context).</summary>
    public static IReadOnlyList<string> Rung1CandidateOnlyMethods { get; } =
    [
        "B2_Ping",
        "B2_Echo1K",
        "B2_Echo64K",
        "B2_ReadBytesGrpc1M",
        "B2_QueueStub20",
    ];
}

/// <summary>One comparable scenario: baseline method vs candidate method.</summary>
/// <param name="Id">Stable id for machine consumers.</param>
/// <param name="DisplayName">Human label in the comparison table.</param>
/// <param name="BaselineMethod">Workload method name for the baseline.</param>
/// <param name="CandidateMethod">Workload method name for the candidate.</param>
/// <param name="MaxMeanOverhead">
/// Max allowed <c>(candidateMean / baselineMean) - 1</c>; <see langword="null"/> = informational only.
/// </param>
internal sealed record ScenarioPair(
    string Id,
    string DisplayName,
    string BaselineMethod,
    string CandidateMethod,
    double? MaxMeanOverhead);

/// <summary>Threshold evaluation outcome for a paired scenario.</summary>
internal enum ScenarioVerdict
{
    /// <summary>No threshold configured; ratio shown for context only.</summary>
    Informational,

    /// <summary>Within the configured mean overhead budget.</summary>
    Pass,

    /// <summary>Over budget but within 1.5× the allowed overhead (or soft competitive miss).</summary>
    Watch,

    /// <summary>Clearly over the decision threshold.</summary>
    Fail,

    /// <summary>One or both methods missing from this run (e.g. filtered).</summary>
    Missing,
}
