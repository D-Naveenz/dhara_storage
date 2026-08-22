using Dhara.Storage.Core;
using Dhara.Storage.Models.Analysis;
using Dhara.Storage.Models.Metadata;
using Dhara.Storage.Runtime;
using Dhara.Storage.Sd.V1;

namespace Dhara.Storage;

/// <summary>
/// Entry points for creating strongly typed storage wrappers and running direct metadata queries.
/// </summary>
public static class DharaStorage
{
    /// <summary>
    /// Creates a path-based file wrapper.
    /// </summary>
    public static StorageFile File(string path) => new(path);

    /// <summary>
    /// Creates a path-based directory wrapper.
    /// </summary>
    public static StorageDirectory Directory(string path) => new(path);

    /// <summary>
    /// Runs content analysis for a path immediately.
    /// </summary>
    public static AnalysisReport AnalyzePath(string path)
    {
        var response = DaemonClient.Call(
            (client, options) => client.AnalyzePath(new AnalyzePathRequest { Path = path }, options),
            path,
            nameof(DharaSd.DharaSdClient.AnalyzePath));
        return DaemonModelFactory.ToAnalysisReport(response, path);
    }

    /// <summary>
    /// Queries file metadata immediately.
    /// </summary>
    public static FileMetadata GetFileMetadata(
        string path,
        bool includeAnalysis = false,
        bool includeIcon = false,
        int iconSize = 32)
    {
        var response = DaemonClient.Call(
            (client, options) => client.GetFileMetadata(
                new GetFileMetadataRequest
                {
                    Path = path,
                    IncludeAnalysis = includeAnalysis,
                    IncludeIcon = includeIcon,
                    IconSize = checked((uint)iconSize),
                },
                options),
            path,
            nameof(DharaSd.DharaSdClient.GetFileMetadata));
        AnalysisReport? analysis = null;
        if (includeAnalysis && response.Analysis is not null)
        {
            analysis = DaemonModelFactory.ToAnalysisReport(response.Analysis, path);
        }

        return DaemonModelFactory.ToFileMetadata(response, analysis);
    }

    /// <summary>
    /// Queries directory metadata immediately.
    /// </summary>
    public static DirectoryMetadata GetDirectoryMetadata(
        string path,
        bool includeSummary = false,
        bool includeIcon = false,
        int iconSize = 32)
    {
        var response = DaemonClient.Call(
            (client, options) => client.GetDirectoryMetadata(
                new GetDirectoryMetadataRequest
                {
                    Path = path,
                    IncludeSummary = includeSummary,
                    IncludeIcon = includeIcon,
                    IconSize = checked((uint)iconSize),
                },
                options),
            path,
            nameof(DharaSd.DharaSdClient.GetDirectoryMetadata));
        return DaemonModelFactory.ToDirectoryMetadata(response);
    }
}
