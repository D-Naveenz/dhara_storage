namespace Dhara.Storage.Interop.Native;

internal sealed record NativeErrorPayload(
    string Code,
    string Message,
    string? Path,
    string? Operation,
    string? Kind,
    string? Value);

internal sealed record NativeLogRecordDto(
    string Level,
    string Target,
    string Message,
    ulong TimestampUnixMs,
    string? ModulePath,
    string? File,
    uint? Line,
    Dictionary<string, string> Fields);
