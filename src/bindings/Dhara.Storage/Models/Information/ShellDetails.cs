namespace Dhara.Storage.Models.Information;

/// <summary>
/// Windows shell display metadata when available from the native runtime.
/// </summary>
public sealed record ShellDetails(string? DisplayName, string? TypeName);
