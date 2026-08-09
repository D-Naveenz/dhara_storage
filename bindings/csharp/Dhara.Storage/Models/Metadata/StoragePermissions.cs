namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents best-effort effective permissions for the current process on a storage path.
/// </summary>
/// <param name="CanRead">Whether the current process can read the entry.</param>
/// <param name="CanWrite">Whether the current process can write the entry.</param>
/// <param name="CanModify">Whether the current process can modify/replace content (write without read-only).</param>
/// <param name="CanExecute">Whether the current process can execute the entry (files) or search (directories).</param>
/// <remarks>This is not a security boundary (TOCTOU applies) and does not enumerate ACL principals
/// the way a platform security editor does.</remarks>
public sealed record StoragePermissions(bool CanRead, bool CanWrite, bool CanModify, bool CanExecute);
