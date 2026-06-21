using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Information;
using Microsoft.Extensions.Logging;
using Dhara.Storage.Core;

namespace Dhara.Storage;

/// <summary>
/// Entry points for creating strongly typed storage wrappers and running direct metadata queries.
/// </summary>
public static class DharaStorage
{
    /// <summary>
    /// Registers an <see cref="ILoggerFactory"/> that receives both managed wrapper logs and native runtime logs.
    /// </summary>
    /// <remarks>Passing <see langword="null"/> removes the current logger factory and stops forwarding native log records.
    /// Configure logging before starting long-running storage operations when you want initialization, progress, and failure details to flow into the host logging pipeline.</remarks>
    /// <param name="loggerFactory">The logger factory that should receive Dhara Storage log events, or <see langword="null"/> to disable forwarding.</param>
    /// <exception cref="PlatformNotSupportedException">Thrown when logging is configured on a platform other than Windows x64 or Windows arm64.</exception>
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
    /// <exception cref="PlatformNotSupportedException">Thrown when called outside Windows x64 or Windows arm64.</exception>
    public static AnalysisReport AnalyzePath(string path) => Interop.Native.NativeQueryInvoker.AnalyzePath(path);

    /// <summary>
    /// Queries file information immediately.
    /// </summary>
    /// <param name="path">The file path to inspect.</param>
    /// <param name="includeAnalysis"><see langword="true"/> to include content-analysis results in the returned snapshot; otherwise, <see langword="false"/> to load metadata only.</param>
    /// <returns>A <see cref="FileInformation"/> snapshot for <paramref name="path"/>.</returns>
    /// <exception cref="PlatformNotSupportedException">Thrown when called outside Windows x64 or Windows arm64.</exception>
    public static FileInformation GetFileInformation(string path, bool includeAnalysis = false) =>
        Interop.Native.NativeQueryInvoker.GetFileInformation(path, includeAnalysis);

    /// <summary>
    /// Queries directory information immediately.
    /// </summary>
    /// <param name="path">The directory path to inspect.</param>
    /// <param name="includeSummary"><see langword="true"/> to include recursive size and entry counts in the returned snapshot; otherwise, <see langword="false"/> to load metadata only.</param>
    /// <returns>A <see cref="DirectoryInformation"/> snapshot for <paramref name="path"/>.</returns>
    /// <exception cref="PlatformNotSupportedException">Thrown when called outside Windows x64 or Windows arm64.</exception>
    public static DirectoryInformation GetDirectoryInformation(string path, bool includeSummary = false) =>
        Interop.Native.NativeQueryInvoker.GetDirectoryInformation(path, includeSummary);
}
