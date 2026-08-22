namespace Dhara.Storage.Models.Metadata;

/// <summary>
/// Represents settable filesystem attributes shared by files and directories.
/// </summary>
/// <param name="ReadOnly">Read-only bit / owner write disabled.</param>
/// <param name="Hidden">Hidden (Windows attribute, or Unix leading-dot name convention on read).</param>
/// <param name="System">Windows system attribute; no portable Unix equivalent.</param>
/// <remarks>Temporary and symbolic-link state are not attributes here: they are creation-time /
/// detection concerns reported separately on <see cref="StorageMetadata"/>.</remarks>
public sealed record StorageAttributes(bool ReadOnly, bool Hidden, bool System);
