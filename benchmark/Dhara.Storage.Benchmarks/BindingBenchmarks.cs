using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Jobs;
using Dhara.Storage.Sd.V1;
using Google.Protobuf;
// using Microsoft.VSDiagnostics; // CPUUsageDiagnoser: enable when ETW/DiagnosticsHub is available
using Microsoft.Win32.SafeHandles;

namespace Dhara.Storage.Benchmarks;

/// <summary>
/// BenchmarkDotNet suite: in-process FFI (B1) vs dhara-sd daemon (B2).
/// </summary>
[MemoryDiagnoser]
// [CPUUsageDiagnoser] // Requires DiagnosticsHub ETW; comment back in when sessions are free
[SimpleJob(RuntimeMoniker.Net10_0, warmupCount: 1, iterationCount: 8)]
public class BindingBenchmarks
{
    private const int RpcDeadlineSeconds = 30;

    private DaemonHost? _host;
    private string _file4K = string.Empty;
    private string _file1M = string.Empty;
    private string _listDir = string.Empty;
    private string _analyzePath = string.Empty;
    private string _copySource = string.Empty;
    private string _copyDestB1 = string.Empty;
    private string _copyDestB2 = string.Empty;
    private string _writeDestB1 = string.Empty;
    private string _writeDestB2 = string.Empty;
    private byte[] _writePayload = [];
    private ByteString _echo1K = ByteString.Empty;
    private ByteString _echo64K = ByteString.Empty;

    /// <summary>Starts the dhara-sd daemon and prepares fixtures once per process.</summary>
    [GlobalSetup]
    public void Setup()
    {
        if (!OperatingSystem.IsWindows())
        {
            throw new PlatformNotSupportedException("Binding benchmarks are Windows-only.");
        }

        _host = DaemonHost.StartAsync().GetAwaiter().GetResult();
        _file4K = FixtureFactory.EnsureSizedFile("bdn-4k.bin", 4 * 1024);
        _file1M = FixtureFactory.EnsureSizedFile("bdn-1m.bin", 1024 * 1024);
        _listDir = FixtureFactory.EnsureListingDirectory("bdn-list-100", 100);
        _analyzePath = FixtureFactory.CoreFixture("sample-2.pdf") ?? _file4K;
        _copySource = FixtureFactory.EnsureSizedFile("bdn-copy-src-1m.bin", 1024 * 1024);
        _copyDestB1 = Path.Combine(FixtureFactory.Root, "bdn-copy-b1.bin");
        _copyDestB2 = Path.Combine(FixtureFactory.Root, "bdn-copy-b2.bin");
        _writeDestB1 = Path.Combine(FixtureFactory.Root, "bdn-write-b1.bin");
        _writeDestB2 = Path.Combine(FixtureFactory.Root, "bdn-write-b2.bin");
        _writePayload = new byte[1024 * 1024];
        Random.Shared.NextBytes(_writePayload);
        _echo1K = ByteString.CopyFrom(new byte[1024]);
        _echo64K = ByteString.CopyFrom(new byte[64 * 1024]);
    }

    /// <summary>Stops the dhara-sd daemon after all benchmarks complete.</summary>
    [GlobalCleanup]
    public void Cleanup()
    {
        _host?.DisposeAsync().AsTask().GetAwaiter().GetResult();
        _host = null;
    }

    private DharaSd.DharaSdClient Client =>
        _host?.Client ?? throw new InvalidOperationException("Daemon host was not started.");

    /// <summary>
    /// Absolute gRPC deadline (not a discarded <see cref="CancellationTokenSource"/> token —
    /// those cancel when the source is GC'd mid-BDN iteration).
    /// </summary>
    private static DateTime RpcDeadlineUtc() =>
        DateTime.UtcNow.AddSeconds(RpcDeadlineSeconds);

    [Benchmark(Description = "B2 Ping")]
    public async Task B2_Ping() =>
        _ = await Client.PingAsync(new PingRequest(), deadline: RpcDeadlineUtc()).ConfigureAwait(false);

    [Benchmark(Description = "B2 Echo 1KB")]
    public async Task B2_Echo1K() =>
        _ = await Client.EchoAsync(new EchoRequest { Payload = _echo1K }, deadline: RpcDeadlineUtc()).ConfigureAwait(false);

    [Benchmark(Description = "B2 Echo 64KB")]
    public async Task B2_Echo64K() =>
        _ = await Client.EchoAsync(new EchoRequest { Payload = _echo64K }, deadline: RpcDeadlineUtc()).ConfigureAwait(false);

    [Benchmark(Description = "B1 GetFileMetadata")]
    public void B1_GetFileMetadata() =>
        FfiBaseline.GetFileMetadata(_file4K);

    [Benchmark(Description = "B2 GetFileMetadata")]
    public async Task B2_GetFileMetadata() =>
        _ = await Client.GetFileMetadataAsync(new GetFileMetadataRequest { Path = _file4K }, deadline: RpcDeadlineUtc()).ConfigureAwait(false);

    [Benchmark(Description = "B1 ListEntries/100")]
    public void B1_ListEntries100() =>
        FfiBaseline.ListEntries(_listDir);

    [Benchmark(Description = "B2 ListEntries/100")]
    public async Task B2_ListEntries100() =>
        _ = await Client.ListEntriesAsync(new ListEntriesRequest { Path = _listDir }, deadline: RpcDeadlineUtc()).ConfigureAwait(false);

    [Benchmark(Description = "B1 AnalyzePath")]
    public void B1_AnalyzePath() =>
        FfiBaseline.AnalyzePath(_analyzePath);

    [Benchmark(Description = "B2 AnalyzePath")]
    public async Task B2_AnalyzePath() =>
        _ = await Client.AnalyzePathAsync(new AnalyzePathRequest { Path = _analyzePath }, deadline: RpcDeadlineUtc()).ConfigureAwait(false);

    [Benchmark(Description = "B1 Read 4KB")]
    public void B1_Read4K() =>
        FfiBaseline.ReadBytes(_file4K);

    [Benchmark(Description = "B2 ReadHandleDup 4KB")]
    public void B2_ReadHandleDup4K() =>
        ReadViaDuplicatedHandle(_file4K);

    [Benchmark(Description = "B1 Read 1MB")]
    public void B1_Read1M() =>
        FfiBaseline.ReadBytes(_file1M);

    [Benchmark(Description = "B2 ReadHandleDup 1MB")]
    public void B2_ReadHandleDup1M() =>
        ReadViaDuplicatedHandle(_file1M);

    [Benchmark(Description = "B2 ReadBytesGrpc 1MB")]
    public async Task B2_ReadBytesGrpc1M() =>
        _ = await Client.ReadFileBytesAsync(new ReadFileBytesRequest { Path = _file1M }, deadline: RpcDeadlineUtc()).ConfigureAwait(false);

    [Benchmark(Description = "B1 Write 1MB")]
    public void B1_Write1M()
    {
        TryDelete(_writeDestB1);
        FfiBaseline.WriteBytes(_writeDestB1, _writePayload);
    }

    [Benchmark(Description = "B2 WriteHandleDup 1MB")]
    public void B2_WriteHandleDup1M()
    {
        TryDelete(_writeDestB2);
        WriteViaDuplicatedHandle(_writeDestB2, _writePayload);
    }

    [Benchmark(Description = "B1 Copy 1MB")]
    public void B1_Copy1M()
    {
        TryDelete(_copyDestB1);
        FfiBaseline.CopyFile(_copySource, _copyDestB1);
    }

    [Benchmark(Description = "B2 Copy 1MB")]
    public async Task B2_Copy1M()
    {
        TryDelete(_copyDestB2);
        using var call = Client.CopyFile(
            new CopyFileRequest
            {
                Source = _copySource,
                Destination = _copyDestB2,
                Overwrite = true,
            },
            deadline: RpcDeadlineUtc());
        while (await call.ResponseStream.MoveNext(CancellationToken.None).ConfigureAwait(false))
        {
            if (call.ResponseStream.Current.Completed)
            {
                if (!string.IsNullOrEmpty(call.ResponseStream.Current.ErrorMessage))
                {
                    throw new InvalidOperationException(call.ResponseStream.Current.ErrorMessage);
                }

                break;
            }
        }
    }

    [Benchmark(Description = "B2 QueueStub 20 jobs")]
    public async Task B2_QueueStub20()
    {
        var deadline = RpcDeadlineUtc();
        for (var i = 0; i < 20; i++)
        {
            _ = await Client.EnqueueWorkAsync(
                new EnqueueWorkRequest
                {
                    Kind = "analyze",
                    Path = _file4K,
                },
                deadline: deadline).ConfigureAwait(false);
        }

        using var stream = Client.StreamWorkEvents(
            new StreamWorkEventsRequest { SyntheticCount = 20 },
            deadline: deadline);
        var received = 0;
        while (await stream.ResponseStream.MoveNext(CancellationToken.None).ConfigureAwait(false))
        {
            received++;
            if (received >= 20)
            {
                break;
            }
        }
    }

    private void ReadViaDuplicatedHandle(string path)
    {
        var opened = Client.OpenReadHandle(new OpenReadHandleRequest { Path = path }, deadline: RpcDeadlineUtc());
        using var safe = new SafeFileHandle((nint)opened.Handle, ownsHandle: true);
        using var stream = new FileStream(safe, FileAccess.Read, bufferSize: 64 * 1024, isAsync: false);
        var buffer = new byte[64 * 1024];
        while (stream.Read(buffer, 0, buffer.Length) > 0)
        {
        }
    }

    private void WriteViaDuplicatedHandle(string path, byte[] payload)
    {
        var opened = Client.OpenWriteHandle(
            new OpenWriteHandleRequest
            {
                Path = path,
                Overwrite = true,
                CreateParentDirectories = true,
            },
            deadline: RpcDeadlineUtc());
        using var safe = new SafeFileHandle((nint)opened.Handle, ownsHandle: true);
        using var stream = new FileStream(safe, FileAccess.Write, bufferSize: 64 * 1024, isAsync: false);
        stream.Write(payload, 0, payload.Length);
        stream.Flush();
    }

    private static void TryDelete(string path)
    {
        if (File.Exists(path))
        {
            File.Delete(path);
        }
    }
}

/// <summary>Short local-iteration job (<c>--filter *Smoke*</c> or <c>--smoke</c>).</summary>
[MemoryDiagnoser]
[SimpleJob(RuntimeMoniker.Net10_0, warmupCount: 0, iterationCount: 3, invocationCount: 16)]
public class BindingSmokeBenchmarks
{
    private DaemonHost? _host;
    private string _file4K = string.Empty;

    [GlobalSetup]
    public void Setup()
    {
        _host = DaemonHost.StartAsync().GetAwaiter().GetResult();
        _file4K = FixtureFactory.EnsureSizedFile("bdn-smoke-4k.bin", 4096);
    }

    [GlobalCleanup]
    public void Cleanup() =>
        _host?.DisposeAsync().AsTask().GetAwaiter().GetResult();

    [Benchmark]
    public async Task Ping() =>
        _ = await _host!.Client.PingAsync(new PingRequest()).ConfigureAwait(false);

    [Benchmark]
    public void GetFileMetadata_B1() =>
        FfiBaseline.GetFileMetadata(_file4K);

    [Benchmark]
    public async Task GetFileMetadata_B2() =>
        _ = await _host!.Client.GetFileMetadataAsync(new GetFileMetadataRequest { Path = _file4K }).ConfigureAwait(false);
}
