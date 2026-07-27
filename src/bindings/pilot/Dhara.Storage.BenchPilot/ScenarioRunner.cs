using System.Diagnostics;
using Dhara.Storage.Models.Progress;
using Dhara.Storage.Pilot.V1;
using Google.Protobuf;
using Microsoft.Win32.SafeHandles;

namespace Dhara.Storage.BenchPilot;

/// <summary>
/// Runs the Phase 0–3 scenario matrix against B1 (FFI) and B2 (daemon).
/// </summary>
internal static class ScenarioRunner
{
    public static async Task<BenchReport> RunSmokeAsync(DaemonHost host)
    {
        var report = new BenchReport();
        var client = host.Client;
        var sample = FixtureFactory.EnsureSizedFile("smoke-4k.bin", 4096);

        report.Scenarios.Add(await Timing.MeasureAsync(
            "Smoke/Ping",
            "B2",
            50,
            async () => { _ = await client.PingAsync(new PingRequest()).ConfigureAwait(false); }).ConfigureAwait(false));

        report.Scenarios.Add(Timing.Measure(
            "Smoke/GetFileInfo",
            "B1",
            50,
            () => { _ = DharaStorage.GetFileInformation(sample); }));

        report.Scenarios.Add(await Timing.MeasureAsync(
            "Smoke/GetFileInfo",
            "B2",
            50,
            async () =>
            {
                _ = await client.GetFileInfoAsync(new GetFileInfoRequest { Path = sample }).ConfigureAwait(false);
            }).ConfigureAwait(false));

        report.Scenarios.Add(Timing.Measure(
            "Smoke/Read",
            "B1",
            10,
            () => { _ = DharaStorage.File(sample).ReadBytes(); },
            payloadBytes: 4096));

        report.Scenarios.Add(await Timing.MeasureAsync(
            "Smoke/ReadHandleDup",
            "B2",
            10,
            async () => { await ReadViaDuplicatedHandleAsync(client, sample).ConfigureAwait(false); },
            payloadBytes: 4096).ConfigureAwait(false));

        report.Recommendation = "Smoke run only — use --quick or --full for the complete matrix.";
        return report;
    }

    public static async Task<BenchReport> RunAsync(DaemonHost host, bool quick)
    {
        var report = new BenchReport();
        var client = host.Client;

        Console.WriteLine("Phase 0 — transport floor");
        await AddPhase0Async(report, client, quick).ConfigureAwait(false);

        Console.WriteLine("Phase 1 — operation parity");
        await AddPhase1Async(report, client, quick).ConfigureAwait(false);

        Console.WriteLine("Phase 2 — lifecycle");
        if (!quick)
        {
            await AddPhase2Async(report, quick).ConfigureAwait(false);
        }

        Console.WriteLine("Phase 3 — personas");
        await AddPersonasAsync(report, client, quick).ConfigureAwait(false);

        report.Recommendation = Recommend(report);
        return report;
    }

    private static async Task AddPhase0Async(BenchReport report, DharaPilot.DharaPilotClient client, bool quick)
    {
        var pingIters = quick ? 200 : 1000;
        report.Scenarios.Add(await Timing.MeasureAsync(
            "Ping",
            "B2",
            pingIters,
            async () => { _ = await client.PingAsync(new PingRequest()).ConfigureAwait(false); }).ConfigureAwait(false));

        foreach (var size in new[] { 1024, 64 * 1024, 1024 * 1024 })
        {
            if (quick && size > 64 * 1024)
            {
                continue;
            }

            var payload = ByteString.CopyFrom(new byte[size]);
            var iters = quick ? 20 : (size >= 1024 * 1024 ? 20 : 100);
            report.Scenarios.Add(await Timing.MeasureAsync(
                $"Echo/{FormatSize(size)}",
                "B2",
                iters,
                async () =>
                {
                    _ = await client.EchoAsync(new EchoRequest { Payload = payload }).ConfigureAwait(false);
                },
                payloadBytes: size).ConfigureAwait(false));
        }
    }

    private static async Task AddPhase1Async(BenchReport report, DharaPilot.DharaPilotClient client, bool quick)
    {
        var small = FixtureFactory.EnsureSizedFile("meta-4k.bin", 4 * 1024);
        Console.WriteLine("  Metadata + analysis");
        var analyzeFile = FixtureFactory.CoreFixture("sample-2.pdf") ?? small;

        report.Scenarios.Add(Timing.Measure(
            "GetFileInfo",
            "B1",
            quick ? 100 : 1000,
            () => { _ = DharaStorage.GetFileInformation(small); }));

        report.Scenarios.Add(await Timing.MeasureAsync(
            "GetFileInfo",
            "B2",
            quick ? 100 : 1000,
            async () =>
            {
                _ = await client.GetFileInfoAsync(new GetFileInfoRequest { Path = small }).ConfigureAwait(false);
            }).ConfigureAwait(false));

        foreach (var count in quick ? new[] { 10, 100 } : new[] { 10, 100, 1000, 10_000 })
        {
            var dir = FixtureFactory.EnsureListingDirectory($"list-{count}", count);
            var listIters = quick ? 3 : 20;

            report.Scenarios.Add(Timing.Measure(
                $"ListEntries/{count}",
                "B1",
                listIters,
                () => { _ = DharaStorage.Directory(dir).GetEntries(); }));

            report.Scenarios.Add(await Timing.MeasureAsync(
                $"ListEntries/{count}",
                "B2",
                listIters,
                async () =>
                {
                    _ = await client.ListEntriesAsync(new ListEntriesRequest { Path = dir }).ConfigureAwait(false);
                }).ConfigureAwait(false));
        }

        report.Scenarios.Add(Timing.Measure(
            "AnalyzePath",
            "B1",
            quick ? 5 : 100,
            () => { _ = DharaStorage.AnalyzePath(analyzeFile); }));

        report.Scenarios.Add(await Timing.MeasureAsync(
            "AnalyzePath",
            "B2",
            quick ? 5 : 100,
            async () =>
            {
                _ = await client.AnalyzePathAsync(new AnalyzePathRequest { Path = analyzeFile }).ConfigureAwait(false);
            }).ConfigureAwait(false));

        var readSizes = quick
            ? new long[] { 4 * 1024 }
            : new long[] { 4 * 1024, 1 * 1024 * 1024, 16 * 1024 * 1024, 64 * 1024 * 1024, 256 * 1024 * 1024 };

        foreach (var size in readSizes)
        {
            var path = FixtureFactory.EnsureSizedFile($"read-{size}.bin", size);
            var iters = size >= 64 * 1024 * 1024 ? 1 : (quick ? 3 : 5);

            Console.WriteLine($"  Read matrix {FormatSize(size)}");
            report.Scenarios.Add(Timing.Measure(
                $"Read/{FormatSize(size)}",
                "B1",
                iters,
                () => { _ = DharaStorage.File(path).ReadBytes(); },
                payloadBytes: size));

            if (!quick)
            {
                report.Scenarios.Add(await Timing.MeasureAsync(
                    $"ReadBytesRpc/{FormatSize(size)}",
                    "B2",
                    iters,
                    async () =>
                    {
                        _ = await client.ReadFileBytesAsync(new ReadFileBytesRequest { Path = path }).ConfigureAwait(false);
                    },
                    payloadBytes: size,
                    notes: "bytes over gRPC").ConfigureAwait(false));
            }

            report.Scenarios.Add(await Timing.MeasureAsync(
                $"ReadHandleDup/{FormatSize(size)}",
                "B2",
                iters,
                async () => { await ReadViaDuplicatedHandleAsync(client, path).ConfigureAwait(false); },
                payloadBytes: size,
                notes: "DuplicateHandle + FileStream").ConfigureAwait(false));
        }

        var writeSize = quick ? 1 * 1024 * 1024L : 64 * 1024 * 1024L;
        var writeBytes = new byte[writeSize];
        Random.Shared.NextBytes(writeBytes);
        var writeDestB1 = Path.Combine(FixtureFactory.Root, "write-b1.bin");
        var writeDestB2 = Path.Combine(FixtureFactory.Root, "write-b2.bin");

        report.Scenarios.Add(await Timing.MeasureAsync(
            $"Write/{FormatSize(writeSize)}",
            "B1",
            quick ? 2 : 3,
            async () =>
            {
                await DharaStorage.File(writeDestB1).WriteAsync(writeBytes, overwrite: true).ConfigureAwait(false);
            },
            payloadBytes: writeSize).ConfigureAwait(false));

        report.Scenarios.Add(await Timing.MeasureAsync(
            $"Write/{FormatSize(writeSize)}",
            "B2",
            quick ? 2 : 3,
            async () =>
            {
                _ = await client.WriteFileBytesAsync(new WriteFileBytesRequest
                {
                    Path = writeDestB2,
                    Data = ByteString.CopyFrom(writeBytes),
                    Overwrite = true,
                    CreateParentDirectories = true,
                }).ConfigureAwait(false);
            },
            payloadBytes: writeSize).ConfigureAwait(false));

        await SafeAddCopyAndWatchAsync(report, client, quick).ConfigureAwait(false);
    }

    private static async Task SafeAddCopyAndWatchAsync(
        BenchReport report,
        DharaPilot.DharaPilotClient client,
        bool quick)
    {
        try
        {
            await AddCopyAndWatchAsync(report, client, quick).ConfigureAwait(false);
        }
        catch (Exception ex)
        {
            report.Scenarios.Add(new ScenarioResult
            {
                Name = "Copy/Watch",
                Baseline = "B2",
                Iterations = 0,
                Notes = $"failed: {ex.Message}",
            });
        }
    }

    private static async Task AddCopyAndWatchAsync(
        BenchReport report,
        DharaPilot.DharaPilotClient client,
        bool quick)
    {
        var copySize = quick ? 1 * 1024 * 1024L : 100 * 1024 * 1024L;
        var source = FixtureFactory.EnsureSizedFile($"copy-src-{copySize}.bin", copySize);
        var destB1 = Path.Combine(FixtureFactory.Root, "copy-b1.bin");
        var destB2 = Path.Combine(FixtureFactory.Root, "copy-b2.bin");
        TryDelete(destB1);
        TryDelete(destB2);

        var progressLatenciesB1 = new List<double>();
        var b1Sw = Stopwatch.StartNew();
        var halfwayB1 = Stopwatch.StartNew();
        var sawHalfB1 = false;
        var localProgress = new Progress<StorageProgress>(p =>
        {
            if (!sawHalfB1 && p.TotalBytes is > 0 && p.BytesTransferred * 2 >= p.TotalBytes.Value)
            {
                sawHalfB1 = true;
                progressLatenciesB1.Clear();
                progressLatenciesB1.Add(halfwayB1.Elapsed.TotalMilliseconds);
            }
        });
        if (quick)
        {
            await DharaStorage.File(source).CopyAsync(destB1, overwrite: true).ConfigureAwait(false);
        }
        else
        {
            await DharaStorage.File(source).CopyAsync(destB1, overwrite: true, progress: localProgress)
                .ConfigureAwait(false);
        }

        b1Sw.Stop();
        report.Scenarios.Add(new ScenarioResult
        {
            Name = $"Copy/{FormatSize(copySize)}",
            Baseline = "B1",
            Iterations = 1,
            ElapsedMs = b1Sw.Elapsed.TotalMilliseconds,
            P50Ms = b1Sw.Elapsed.TotalMilliseconds,
            P95Ms = b1Sw.Elapsed.TotalMilliseconds,
            ThroughputMBps = copySize / (1024.0 * 1024.0) / Math.Max(b1Sw.Elapsed.TotalSeconds, 1e-9),
            Notes = sawHalfB1
                ? $"half-progress-visible-ms={progressLatenciesB1[0]:F2}"
                : "half-progress-not-observed",
        });

        var halfVisibleB2 = 0.0;
        var sawHalfB2 = false;
        var halfwayB2 = Stopwatch.StartNew();
        var b2Sw = Stopwatch.StartNew();
        using var call = client.CopyFile(new CopyFileRequest
        {
            Source = source,
            Destination = destB2,
            Overwrite = true,
        });
        try
        {
            (sawHalfB2, halfVisibleB2) = await ConsumeCopyStreamAsync(call, halfwayB2)
                .WaitAsync(TimeSpan.FromSeconds(quick ? 30 : 120))
                .ConfigureAwait(false);
        }
        catch (TimeoutException)
        {
            report.Scenarios.Add(new ScenarioResult
            {
                Name = $"Copy/{FormatSize(copySize)}",
                Baseline = "B2",
                Iterations = 1,
                ElapsedMs = b2Sw.Elapsed.TotalMilliseconds,
                Notes = "copy stream timed out",
            });
            return;
        }

        b2Sw.Stop();
        report.Scenarios.Add(new ScenarioResult
        {
            Name = $"Copy/{FormatSize(copySize)}",
            Baseline = "B2",
            Iterations = 1,
            ElapsedMs = b2Sw.Elapsed.TotalMilliseconds,
            P50Ms = b2Sw.Elapsed.TotalMilliseconds,
            P95Ms = b2Sw.Elapsed.TotalMilliseconds,
            ThroughputMBps = copySize / (1024.0 * 1024.0) / Math.Max(b2Sw.Elapsed.TotalSeconds, 1e-9),
            Notes = sawHalfB2
                ? $"half-progress-visible-ms={halfVisibleB2:F2}"
                : "half-progress-not-observed",
        });

        // Watch: create N files and measure event delivery latency (B2 stream).
        var watchDir = Path.Combine(FixtureFactory.Root, "watch-b2");
        Directory.CreateDirectory(watchDir);
        foreach (var file in Directory.EnumerateFiles(watchDir))
        {
            File.Delete(file);
        }

        var eventCount = quick ? 5 : 100;
        using var watchCall = client.WatchDirectory(new WatchDirectoryRequest
        {
            Path = watchDir,
            Recursive = false,
            DebounceWindowMs = 25,
        });

        var watchTask = Task.Run(async () =>
        {
            var received = 0;
            var firstLatency = 0.0;
            var started = Stopwatch.StartNew();
            while (received < eventCount
                   && await watchCall.ResponseStream.MoveNext(CancellationToken.None).ConfigureAwait(false))
            {
                received++;
                if (received == 1)
                {
                    firstLatency = started.Elapsed.TotalMilliseconds;
                }
            }

            return (received, firstLatency, started.Elapsed.TotalMilliseconds);
        });

        await Task.Delay(150).ConfigureAwait(false);
        var createSw = Stopwatch.StartNew();
        for (var i = 0; i < eventCount; i++)
        {
            await File.WriteAllTextAsync(Path.Combine(watchDir, $"w-{i}.txt"), "x").ConfigureAwait(false);
        }

        createSw.Stop();
        try
        {
            var (receivedCount, firstLatencyMs, totalWatchMs) = await watchTask
                .WaitAsync(TimeSpan.FromSeconds(quick ? 15 : 30))
                .ConfigureAwait(false);
            report.Scenarios.Add(new ScenarioResult
            {
                Name = $"Watch/{eventCount}-creates",
                Baseline = "B2",
                Iterations = 1,
                ElapsedMs = totalWatchMs,
                P50Ms = firstLatencyMs,
                P95Ms = totalWatchMs,
                Notes = $"received={receivedCount}; create-wall-ms={createSw.Elapsed.TotalMilliseconds:F2}",
            });
        }
        catch (TimeoutException)
        {
            report.Scenarios.Add(new ScenarioResult
            {
                Name = $"Watch/{eventCount}-creates",
                Baseline = "B2",
                Iterations = 1,
                ElapsedMs = 0,
                P50Ms = 0,
                P95Ms = 0,
                Notes = "watch stream timed out",
            });
        }
    }

    private static async Task AddPhase2Async(BenchReport report, bool quick)
    {
        _ = quick;
        var sw = Stopwatch.StartNew();
        await using var host = await DaemonHost.StartAsync().ConfigureAwait(false);
        _ = await host.Client.PingAsync(new PingRequest()).ConfigureAwait(false);
        var analyzePath = FixtureFactory.CoreFixture("sample-2.pdf")
            ?? FixtureFactory.EnsureSizedFile("cold-4k.bin", 4096);
        _ = await host.Client.AnalyzePathAsync(new AnalyzePathRequest { Path = analyzePath }).ConfigureAwait(false);
        sw.Stop();

        report.Scenarios.Add(new ScenarioResult
        {
            Name = "ColdStart/Ping+Analyze",
            Baseline = "B2",
            Iterations = 1,
            ElapsedMs = sw.Elapsed.TotalMilliseconds,
            P50Ms = sw.Elapsed.TotalMilliseconds,
            P95Ms = sw.Elapsed.TotalMilliseconds,
            Notes = $"daemon-pid={host.DaemonProcessId}",
        });
    }

    private static async Task AddPersonasAsync(
        BenchReport report,
        DharaPilot.DharaPilotClient client,
        bool quick)
    {
        var classifyCount = quick ? 3 : 200;
        var files = new List<string>();
        var fixture = FixtureFactory.CoreFixture("sample-2.pdf");
        for (var i = 0; i < classifyCount; i++)
        {
            if (fixture is not null && i == 0)
            {
                files.Add(fixture);
                continue;
            }

            var size = 4 * 1024 + (i % 4) * 4 * 1024;
            files.Add(FixtureFactory.EnsureSizedFile($"classify-{i}.bin", size));
        }

        report.Scenarios.Add(Timing.Measure(
            "Persona/Classifier",
            "B1",
            1,
            () =>
            {
                foreach (var file in files)
                {
                    _ = DharaStorage.AnalyzePath(file);
                }
            },
            notes: $"{classifyCount} analyzes"));

        report.Scenarios.Add(await Timing.MeasureAsync(
            "Persona/Classifier",
            "B2",
            1,
            async () =>
            {
                foreach (var file in files)
                {
                    _ = await client.AnalyzePathAsync(new AnalyzePathRequest { Path = file }).ConfigureAwait(false);
                }
            },
            notes: $"{classifyCount} analyzes").ConfigureAwait(false));

        if (!quick)
        {
            var ingestSize = 50 * 1024 * 1024L;
            var ingestSrc = FixtureFactory.EnsureSizedFile($"ingest-{ingestSize}.bin", ingestSize);
            var ingestDest = Path.Combine(FixtureFactory.Root, "ingest-out.bin");
            var ingestDestB2 = ingestDest + ".b2";
            TryDelete(ingestDest);
            TryDelete(ingestDestB2);

            report.Scenarios.Add(await Timing.MeasureAsync(
                "Persona/Ingest",
                "B1",
                1,
                async () =>
                {
                    await DharaStorage.File(ingestSrc).CopyAsync(ingestDest, overwrite: true).ConfigureAwait(false);
                    _ = DharaStorage.AnalyzePath(ingestDest);
                    _ = DharaStorage.File(ingestDest).ReadBytes();
                },
                notes: $"copy+analyze+read {FormatSize(ingestSize)}").ConfigureAwait(false));

            report.Scenarios.Add(await Timing.MeasureAsync(
                "Persona/Ingest",
                "B2",
                1,
                async () =>
                {
                    using (var call = client.CopyFile(new CopyFileRequest
                    {
                        Source = ingestSrc,
                        Destination = ingestDestB2,
                        Overwrite = true,
                    }))
                    {
                        _ = await ConsumeCopyStreamAsync(call, Stopwatch.StartNew())
                            .WaitAsync(TimeSpan.FromSeconds(120))
                            .ConfigureAwait(false);
                    }

                    _ = await client.AnalyzePathAsync(new AnalyzePathRequest { Path = ingestDestB2 })
                        .ConfigureAwait(false);
                    await ReadViaDuplicatedHandleAsync(client, ingestDestB2, maxBytes: 4096).ConfigureAwait(false);
                },
                notes: $"copy stream+analyze+dup-read-4k {FormatSize(ingestSize)}").ConfigureAwait(false));
        }

        if (!quick)
        {
            var browserCount = 2000;
            var browserDir = FixtureFactory.EnsureListingDirectory($"browser-{browserCount}", browserCount);

            report.Scenarios.Add(Timing.Measure(
                "Persona/Browser",
                "B1",
                1,
                () =>
                {
                    var entries = DharaStorage.Directory(browserDir).GetEntries();
                    var take = Math.Min(50, entries.Count);
                    for (var i = 0; i < take; i++)
                    {
                        if (!entries[i].IsDirectory)
                        {
                            _ = DharaStorage.GetFileInformation(entries[i].Path);
                        }
                    }
                }));

            report.Scenarios.Add(await Timing.MeasureAsync(
                "Persona/Browser",
                "B2",
                1,
                async () =>
                {
                    var listed = await client.ListEntriesAsync(new ListEntriesRequest { Path = browserDir })
                        .ConfigureAwait(false);
                    var take = Math.Min(50, listed.Entries.Count);
                    for (var i = 0; i < take; i++)
                    {
                        if (!listed.Entries[i].IsDirectory)
                        {
                            _ = await client.GetFileInfoAsync(new GetFileInfoRequest { Path = listed.Entries[i].Path })
                                .ConfigureAwait(false);
                        }
                    }
                }).ConfigureAwait(false));

            var queueCount = 50;
            report.Scenarios.Add(await Timing.MeasureAsync(
                "Persona/QueueStub",
                "B2",
                1,
                async () =>
                {
                    for (var i = 0; i < queueCount; i++)
                    {
                        _ = await client.EnqueueWorkAsync(new EnqueueWorkRequest
                        {
                            Kind = "analyze",
                            Path = files[i % files.Count],
                        }).ConfigureAwait(false);
                    }

                    using var stream = client.StreamWorkEvents(new StreamWorkEventsRequest
                    {
                        SyntheticCount = (uint)queueCount,
                    });
                    var received = 0;
                    while (await stream.ResponseStream.MoveNext(CancellationToken.None).ConfigureAwait(false))
                    {
                        received++;
                        if (received >= queueCount)
                        {
                            break;
                        }
                    }
                },
                notes: $"{queueCount} enqueue + stream").ConfigureAwait(false));
        }
    }

    private static void TryDelete(string path)
    {
        if (File.Exists(path))
        {
            File.Delete(path);
        }
    }

    private static async Task<(bool SawHalf, double HalfVisibleMs)> ConsumeCopyStreamAsync(
        Grpc.Core.AsyncServerStreamingCall<CopyFileProgress> call,
        Stopwatch halfway)
    {
        var sawHalf = false;
        var halfVisibleMs = 0.0;
        while (await call.ResponseStream.MoveNext(CancellationToken.None).ConfigureAwait(false))
        {
            var msg = call.ResponseStream.Current;
            if (!sawHalf
                && msg.HasTotalBytes
                && msg.TotalBytes > 0
                && msg.BytesTransferred * 2 >= msg.TotalBytes)
            {
                sawHalf = true;
                halfVisibleMs = halfway.Elapsed.TotalMilliseconds;
            }

            if (msg.Completed)
            {
                if (!string.IsNullOrEmpty(msg.ErrorMessage))
                {
                    throw new InvalidOperationException(msg.ErrorMessage);
                }

                break;
            }
        }

        return (sawHalf, halfVisibleMs);
    }

    private static async Task ReadViaDuplicatedHandleAsync(
        DharaPilot.DharaPilotClient client,
        string path,
        long? maxBytes = null)
    {
        var opened = await client.OpenReadHandleAsync(new OpenReadHandleRequest { Path = path })
            .ConfigureAwait(false);
        using var safe = new SafeFileHandle((nint)opened.Handle, ownsHandle: true);
        using var stream = new FileStream(safe, FileAccess.Read, bufferSize: 1024 * 1024, isAsync: false);
        var buffer = new byte[1024 * 1024];
        long remaining = maxBytes ?? long.MaxValue;
        while (remaining > 0)
        {
            var toRead = (int)Math.Min(buffer.Length, remaining);
            var read = stream.Read(buffer, 0, toRead);
            if (read == 0)
            {
                break;
            }

            remaining -= read;
        }
    }

    private static string Recommend(BenchReport report)
    {
        static ScenarioResult? Find(BenchReport r, string name, string baseline) =>
            r.Scenarios.LastOrDefault(s => s.Name == name && s.Baseline == baseline);

        var listB1 = Find(report, "ListEntries/1000", "B1");
        var listB2 = Find(report, "ListEntries/1000", "B2");
        var analyzeB1 = Find(report, "AnalyzePath", "B1");
        var analyzeB2 = Find(report, "AnalyzePath", "B2");
        var readDup = report.Scenarios.LastOrDefault(s => s.Name.StartsWith("ReadHandleDup/", StringComparison.Ordinal) && s.Baseline == "B2");
        _ = readDup;
        // Prefer largest overlapping read size present.
        var largeReadB1 = report.Scenarios
            .Where(s => s.Baseline == "B1" && s.Name.StartsWith("Read/", StringComparison.Ordinal))
            .OrderByDescending(s => s.ThroughputMBps ?? 0)
            .FirstOrDefault();
        var largeReadDup = report.Scenarios
            .Where(s => s.Baseline == "B2" && s.Name.StartsWith("ReadHandleDup/", StringComparison.Ordinal))
            .OrderByDescending(s => s.ThroughputMBps ?? 0)
            .FirstOrDefault();
        var largeReadRpc = report.Scenarios
            .Where(s => s.Baseline == "B2" && s.Name.StartsWith("ReadBytesRpc/", StringComparison.Ordinal))
            .OrderByDescending(s => s.ElapsedMs)
            .FirstOrDefault();

        var lines = new List<string>
        {
            "Auto-derived from this run (confirm against docs/binding-pilot-benchmarks.md thresholds):",
        };

        if (listB1 is not null && listB2 is not null)
        {
            var delta = (listB2.P50Ms - listB1.P50Ms) / Math.Max(listB1.P50Ms, 1e-9) * 100.0;
            lines.Add($"- ListEntries/1000 B2 vs B1 p50: {delta:+0.0;-0.0}%" + (Math.Abs(delta) <= 15 ? " (within +15% budget)" : " (outside +15% budget)"));
        }

        if (analyzeB1 is not null && analyzeB2 is not null)
        {
            var delta = (analyzeB2.P50Ms - analyzeB1.P50Ms) / Math.Max(analyzeB1.P50Ms, 1e-9) * 100.0;
            lines.Add($"- AnalyzePath B2 vs B1 p50: {delta:+0.0;-0.0}%" + (Math.Abs(delta) <= 10 ? " (within +10% budget)" : " (outside +10% budget)"));
        }

        if (largeReadB1?.ThroughputMBps is double b1Tp && largeReadDup?.ThroughputMBps is double dupTp)
        {
            lines.Add($"- Large-read throughput B1={b1Tp:F2} MB/s, B2-dup={dupTp:F2} MB/s" + (dupTp >= b1Tp * 0.95 ? " (dup matches/beats B1)" : " (dup behind B1)"));
        }

        if (largeReadRpc is not null && largeReadDup is not null)
        {
            lines.Add($"- Bytes-RPC is expected to lose vs dup: rpc-elapsed={largeReadRpc.ElapsedMs:F0}ms, dup-elapsed={largeReadDup.ElapsedMs:F0}ms");
        }

        var copyB2 = report.Scenarios.LastOrDefault(s => s.Name.StartsWith("Copy/", StringComparison.Ordinal) && s.Baseline == "B2");
        if (copyB2?.Notes?.Contains("half-progress-visible-ms=") == true)
        {
            lines.Add($"- Copy progress visibility: {copyB2.Notes}");
        }

        lines.Add(string.Empty);
        lines.Add("Suggested outcome: review the deltas above. Prefer **hybrid** if chatty metadata exceeds budget but large-read dup and streaming progress look healthy; **go daemon** if all budgets pass; **stay FFI** if dup does not repay the IPC tax.");
        return string.Join(Environment.NewLine, lines);
    }

    private static string FormatSize(long bytes) =>
        bytes switch
        {
            >= 1024 * 1024 => $"{bytes / (1024 * 1024)}MB",
            >= 1024 => $"{bytes / 1024}KB",
            _ => $"{bytes}B",
        };
}
