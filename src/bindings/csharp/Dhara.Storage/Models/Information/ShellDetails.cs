namespace Dhara.Storage.Models.Information;

/// <summary>
/// Windows shell display metadata when available from the native runtime.
/// </summary>
/// <param name="DisplayName">Shell display name for the item, when available.</param>
/// <param name="TypeName">Shell type name (for example a localized “File folder”), when available.</param>
public sealed record ShellDetails(string? DisplayName, string? TypeName);
