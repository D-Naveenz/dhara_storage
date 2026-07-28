using Dhara.Storage.Core;
using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;
using Dhara.Storage.Runtime;
using Dhara.Storage.Sd.V1;
using Microsoft.Extensions.Logging;

namespace Dhara.Storage;

/// <summary>
/// Entry points for creating strongly typed storage wrappers and running direct metadata queries.
/// </summary>
/// <remarks>
/// <para>
/// Prefer these factories from application code. Paths may refer to existing items or future
/// destinations. Content analysis uses the native definition database (not extension-only MIME).
/// </para>
/// <para>
/// Calls are served by the bundled <c>dhara-sd</c> sidecar daemon over a local gRPC transport;
/// the daemon is started on first use.
/// </para>
/// </remarks>
public static class DharaStorage
{
    /// <summary>
    /// Registers an <see cref="ILoggerFactory"/> that receives managed wrapper logs.
    /// </summary>
    /// <remarks>Passing <see langword="null"/> removes the current logger factory. Configure logging
    /// before starting long-running storage operations when you want initialization, progress, and
    /// failure details to flow into the host logging pipeline.</remarks>
    /// <param name="loggerFactory">The logger factory that should receive Dhara Storage log events, or <see langword="null"/> to disable forwarding.</param>
    public static void UseLoggerFactory(ILoggerFactory? loggerFactory) => DharaStorageLogBridge.UseLoggerFactory(loggerFactory);

    /// <summary>
    /// Creates a path-based file wrapper.
    /// </summary>
    /// <param name="path">The file path to wrap. The path may point to an existing file or to a future destination for write operations.</param>
    /// <returns>A new <see cref="StorageFile"/> wrapper for <paramref name="path"/>.</returns>
    public static StorageFile File(string path) => new(path);

    /// <summary>
    /// Creates a path-based directory wrapper.
    /// </summary>
    /// <param name="path">The directory path to wrap. The path may point to an existing directory or to a future destination for create operations.</param>
    /// <returns>A new <see cref="StorageDirectory"/> wrapper for <paramref name="path"/>.</returns>
    public static StorageDirectory Directory(string path) => new(path);

    /// <summary>
    /// Runs content analysis for a path immediately.
    /// </summary>
    /// <param name="path">The file path to analyze.</param>
    /// <returns>An <see cref="AnalysisReport"/> describing the strongest file-type matches for <paramref name="path"/>.</returns>
    /// <exception cref="PlatformNotSupportedException">Thrown when called on an unsupported operating system or process architecture.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the daemon cannot analyze the path.</exception>
    public static AnalysisReport AnalyzePath(string path)
    {
        var response = DaemonClient.Call(
            (client, options) => client.AnalyzePath(new AnalyzePathRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.AnalyzePath));
        return DaemonModelFactory.ToAnalysisReport(response, path);
    }

    /// <summary>
    /// Queries file information immediately.
    /// </summary>
    /// <param name="path">The file path to inspect.</param>
    /// <param name="includeAnalysis"><see langword="true"/> to include content-analysis results in the returned snapshot; otherwise, <see langword="false"/> to load metadata only.</param>
    /// <param name="includeIcon">Reserved for a future shell-icon RPC; icons are not available over the current daemon transport and <see cref="FileInformation.Icon"/> is always <see langword="null"/>.</param>
    /// <param name="iconSize">Reserved alongside <paramref name="includeIcon"/>; currently unused.</param>
    /// <returns>A <see cref="FileInformation"/> snapshot for <paramref name="path"/>.</returns>
    /// <exception cref="PlatformNotSupportedException">Thrown when called on an unsupported operating system or process architecture.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the daemon cannot read file information.</exception>
    public static FileInformation GetFileInformation(
        string path,
        bool includeAnalysis = false,
        bool includeIcon = false,
        int iconSize = 32)
    {
        _ = includeIcon;
        _ = iconSize;
        var response = DaemonClient.Call(
            (client, options) => client.GetFileInfo(new GetFileInfoRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.GetFileInfo));
        var analysis = includeAnalysis ? AnalyzePath(path) : null;
        return DaemonModelFactory.ToFileInformation(response, analysis);
    }

    /// <summary>
    /// Queries directory information immediately.
    /// </summary>
    /// <param name="path">The directory path to inspect.</param>
    /// <param name="includeSummary"><see langword="true"/> to include recursive size and entry counts in the returned snapshot; otherwise, <see langword="false"/> to load metadata only.</param>
    /// <param name="includeIcon">Reserved for a future shell-icon RPC; icons are not available over the current daemon transport and <see cref="DirectoryInformation.Icon"/> is always <see langword="null"/>.</param>
    /// <param name="iconSize">Reserved alongside <paramref name="includeIcon"/>; currently unused.</param>
    /// <returns>A <see cref="DirectoryInformation"/> snapshot for <paramref name="path"/>.</returns>
    /// <exception cref="PlatformNotSupportedException">Thrown when called on an unsupported operating system or process architecture.</exception>
    /// <exception cref="Exceptions.DharaStorageException">Thrown when the daemon cannot read directory information.</exception>
    public static DirectoryInformation GetDirectoryInformation(
        string path,
        bool includeSummary = false,
        bool includeIcon = false,
        int iconSize = 32)
    {
        _ = includeIcon;
        _ = iconSize;
        var response = DaemonClient.Call(
            (client, options) => client.GetDirectoryInfo(new GetDirectoryInfoRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.GetDirectoryInfo));
        var summary = includeSummary ? DaemonModelFactory.BuildDirectorySummary(path) : null;
        return DaemonModelFactory.ToDirectoryInformation(response, summary);
    }
}
