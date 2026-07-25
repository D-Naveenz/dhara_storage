namespace Dhara.Storage.Models.Progress;

/// <summary>
/// Represents progress reported by long-running native storage operations.
/// </summary>
/// <param name="TotalBytes">Expected total bytes when known; otherwise <see langword="null"/>.</param>
/// <param name="BytesTransferred">Bytes transferred so far.</param>
/// <param name="BytesPerSecond">Observed transfer rate in bytes per second.</param>
public sealed record StorageProgress(
    ulong? TotalBytes,
    ulong BytesTransferred,
    double BytesPerSecond);
