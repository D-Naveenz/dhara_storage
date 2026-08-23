namespace Dhara.Storage.Models.Progress;

/// <summary>
/// Process event streamed from a long-running storage operation (Windows-copy-dialog style).
/// </summary>
public abstract record StorageProcessEvent;

/// <summary>Planning finished; totals are known and writing has not started.</summary>
/// <param name="TotalBytes">Aggregate byte size of the planned work.</param>
/// <param name="TotalFiles">Number of file tasks in the planned work.</param>
public sealed record StorageProcessStarted(ulong TotalBytes, ulong TotalFiles) : StorageProcessEvent;

/// <summary>The writer began a new file task.</summary>
/// <param name="Path">Destination path being written.</param>
/// <param name="FileSize">Size of the current file in bytes.</param>
/// <param name="FileIndex">Zero-based index of this file within the process.</param>
public sealed record StorageProcessCurrentItem(string Path, ulong FileSize, ulong FileIndex) : StorageProcessEvent;

/// <summary>Cumulative bytes transferred so far for the whole process.</summary>
/// <param name="BytesTransferred">Bytes transferred across all tasks so far.</param>
public sealed record StorageProcessBytes(ulong BytesTransferred) : StorageProcessEvent;

/// <summary>The process finished successfully.</summary>
/// <param name="Destination">Primary destination path when applicable.</param>
public sealed record StorageProcessCompleted(string? Destination) : StorageProcessEvent;

/// <summary>The process failed after starting.</summary>
/// <param name="Message">Stable, display-oriented failure message.</param>
public sealed record StorageProcessFailed(string Message) : StorageProcessEvent;

/// <summary>Cooperative cancellation was observed.</summary>
/// <param name="Operation">High-level operation label that observed cancellation.</param>
public sealed record StorageProcessCancelled(string Operation) : StorageProcessEvent;
