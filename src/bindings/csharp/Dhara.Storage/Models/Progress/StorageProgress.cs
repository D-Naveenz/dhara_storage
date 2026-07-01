namespace Dhara.Storage.Models.Progress;

/// <summary>
/// Represents progress reported by long-running native storage operations.
/// </summary>
public sealed record StorageProgress(
    ulong? TotalBytes,
    ulong BytesTransferred,
    double BytesPerSecond);
