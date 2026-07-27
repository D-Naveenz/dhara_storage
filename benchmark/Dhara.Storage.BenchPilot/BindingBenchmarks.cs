using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Jobs;
using Dhara.Storage.Pilot.V1;
using Google.Protobuf;
using Microsoft.Win32.SafeHandles;

namespace Dhara.Storage.BenchPilot;

/// <summary>
/// BenchmarkDotNet suite: in-process FFI (B1) vs pilot daemon (B2).
/// </summary>
[MemoryDiagnoser]
[SimpleJob(RuntimeMoniker.Net10_0, warmupCount: 1, iterationCount: 8)]
public class BindingBenchmarks
{
    private DaemonHost? _host;
    private string _file4K = string.Empty;
    private string _file1M = string.Empty;
    private string _listDir = string.Empty;
    private string _analyzePath = string.Empty;
    private string _copySource = string.Empty;
    private string _copyDestB1 = string.Empty;
    private string _writeDestB1 = string.Empty;
    private byte[] _writePayload = [];
    private ByteString _echo1K = ByteString.Empty;
    private ByteString _echo64K = ByteString.Empty;

    /// <summary>Starts the pilot daemon and prepares fixtures once per process.</summary>
    [GlobalSetup]
    public void Setup()
    {
        if (!OperatingSystem.IsWindows())
        {
            throw new PlatformNotSupportedException("Binding pilot benchmarks are Windows-only.");
        }

        _host = DaemonHost.StartAsync().GetAwaiter().GetResult();
        _file4K = FixtureFactory.EnsureSizedFile("bdn-4k.bin", 4 * 1024);
        _file1M = FixtureFactory.EnsureSizedFile("bdn-1m.bin", 1024 * 1024);
        _listDir = FixtureFactory.EnsureListingDirectory("bdn-list-100", 100);
        _analyzePath = FixtureFactory.CoreFixture("sample-2.pdf") ?? _file4K;
        _copySource = FixtureFactory.EnsureSizedFile("bdn-copy-src-1m.bin", 1024 * 1024);
        _copyDestB1 = Path.Combine(FixtureFactory.Root, "bdn-copy-b1.bin");
        _writeDestB1 = Path.Combine(FixtureFactory.Root, "bdn-write-b1.bin");
        _writePayload = new byte[1024 * 1024];
        Random.Shared.NextBytes(_writePayload);
        _echo1K = ByteString.CopyFrom(new byte[1024]);
        _echo64K = ByteString.CopyFrom(new byte[64 * 1024]);
    }

    /// <summary>Stops the pilot daemon after all benchmarks complete.</summary>
    [GlobalCleanup]
    public void Cleanup()
    {
        _host?.DisposeAsync().AsTask().GetAwaiter().GetResult();
        _host = null;
    }

    private DharaPilot.DharaPilotClient Client =>
        _host?.Client ?? throw new InvalidOperationException("Daemon host was not started.");

    [Benchmark(Description = "B2 Ping")]
    public async Task B2_Ping() =>
        _ = await Client.PingAsync(new PingRequest()).ConfigureAwait(false);

    [Benchmark(Description = "B2 Echo 1KB")]
    public async Task B2_Echo1K() =>
        _ = await Client.EchoAsync(new EchoRequest { Payload = _echo1K }).ConfigureAwait(false);

    [Benchmark(Description = "B2 Echo 64KB")]
    public async Task B2_Echo64K() =>
        _ = await Client.EchoAsync(new EchoRequest { Payload = _echo64K }).ConfigureAwait(false);

    [Benchmark(Description = "B1 GetFileInfo")]
    public void B1_GetFileInfo() =>
        _ = DharaStorage.GetFileInformation(_file4K);

    [Benchmark(Description = "B2 GetFileInfo")]
    public async Task B2_GetFileInfo() =>
        _ = await Client.GetFileInfoAsync(new GetFileInfoRequest { Path = _file4K }).ConfigureAwait(false);

    [Benchmark(Description = "B1 ListEntries/100")]
    public void B1_ListEntries100() =>
        _ = DharaStorage.Directory(_listDir).GetEntries();

    [Benchmark(Description = "B2 ListEntries/100")]
    public async Task B2_ListEntries100() =>
        _ = await Client.ListEntriesAsync(new ListEntriesRequest { Path = _listDir }).ConfigureAwait(false);

    // B2 AnalyzePath / B2 WriteFileBytes (1MB) hang under iterative BDN on this
    // workstation (named-pipe gRPC); keep B1 counterparts and other B2 methods.

    [Benchmark(Description = "B1 AnalyzePath")]
    public void B1_AnalyzePath() =>
        _ = DharaStorage.AnalyzePath(_analyzePath);

    [Benchmark(Description = "B1 Read 4KB")]
    public void B1_Read4K() =>
        _ = DharaStorage.File(_file4K).ReadBytes();

    [Benchmark(Description = "B2 ReadHandleDup 4KB")]
    public void B2_ReadHandleDup4K() =>
        ReadViaDuplicatedHandle(_file4K);

    [Benchmark(Description = "B1 Read 1MB")]
    public void B1_Read1M() =>
        _ = DharaStorage.File(_file1M).ReadBytes();

    [Benchmark(Description = "B2 ReadHandleDup 1MB")]
    public void B2_ReadHandleDup1M() =>
        ReadViaDuplicatedHandle(_file1M);

    [Benchmark(Description = "B2 ReadBytesGrpc 1MB")]
    public async Task B2_ReadBytesGrpc1M() =>
        _ = await Client.ReadFileBytesAsync(new ReadFileBytesRequest { Path = _file1M }).ConfigureAwait(false);

    [Benchmark(Description = "B1 Write 1MB")]
    public void B1_Write1M()
    {
        TryDelete(_writeDestB1);
        DharaStorage.File(_writeDestB1).Write(_writePayload, overwrite: true);
    }

    [Benchmark(Description = "B1 Copy 1MB")]
    public void B1_Copy1M()
    {
        TryDelete(_copyDestB1);
        _ = DharaStorage.File(_copySource).Copy(_copyDestB1, overwrite: true);
    }

    [Benchmark(Description = "B2 QueueStub 20 jobs")]
    public async Task B2_QueueStub20()
    {
        for (var i = 0; i < 20; i++)
        {
            _ = await Client.EnqueueWorkAsync(new EnqueueWorkRequest
            {
                Kind = "analyze",
                Path = _file4K,
            }).ConfigureAwait(false);
        }

        using var stream = Client.StreamWorkEvents(new StreamWorkEventsRequest { SyntheticCount = 20 });
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
        var opened = Client.OpenReadHandle(new OpenReadHandleRequest { Path = path });
        using var safe = new SafeFileHandle((nint)opened.Handle, ownsHandle: true);
        using var stream = new FileStream(safe, FileAccess.Read, bufferSize: 64 * 1024, isAsync: false);
        var buffer = new byte[64 * 1024];
        while (stream.Read(buffer, 0, buffer.Length) > 0)
        {
        }
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
    public void GetFileInfo_B1() =>
        _ = DharaStorage.GetFileInformation(_file4K);

    [Benchmark]
    public async Task GetFileInfo_B2() =>
        _ = await _host!.Client.GetFileInfoAsync(new GetFileInfoRequest { Path = _file4K }).ConfigureAwait(false);
}
