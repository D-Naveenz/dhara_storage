using Dhara.Storage.Exceptions;
using Dhara.Storage.Models.Progress;
using System.Runtime.InteropServices;
using Microsoft.Extensions.Logging;

namespace Dhara.Storage.Interop.Native;

internal static class NativeHelpers
{
    internal static byte ToNativeBool(bool value) => value ? (byte)1 : (byte)0;

    internal static void EnsureSupportedPlatform()
    {
        var arch = RuntimeInformation.ProcessArchitecture;
        var is64Bit = arch is Architecture.X64 or Architecture.Arm64;
        if (!is64Bit)
        {
            throw new PlatformNotSupportedException(
                $"Dhara.Storage requires a 64-bit process (x64 or arm64). Current architecture: {arch}.");
        }

        if (OperatingSystem.IsWindows() || OperatingSystem.IsLinux() || OperatingSystem.IsMacOS())
        {
            return;
        }

        throw new PlatformNotSupportedException(
            $"Dhara.Storage is not supported on this operating system. Current platform: {RuntimeInformation.OSDescription}, architecture: {arch}.");
    }

    internal static void ThrowIfFailed(NativeStatus status, nint errorPtr, nuint errorLen)
    {
        if (status == NativeStatus.Ok)
        {
            return;
        }

        var payloadJson = errorPtr == 0 ? null : NativeMemory.ReadUtf8AndFree(errorPtr, errorLen);
        if (string.IsNullOrWhiteSpace(payloadJson))
        {
            throw new DharaStorageException($"Native call failed with status {status}.", status.ToString());
        }

        var payload = NativeJson.Deserialize<NativeErrorPayload>(payloadJson);
        if (string.Equals(payload.Code, "cancelled", StringComparison.OrdinalIgnoreCase))
        {
            throw new OperationCanceledException(payload.Message);
        }

        throw new DharaStorageException(payload.Message, payload.Code, payload.Path, payload.Operation);
    }

    internal static void ThrowIfFailed(NativeStatus status, nint errorPtr, nuint errorLen, CancellationToken cancellationToken)
    {
        try
        {
            ThrowIfFailed(status, errorPtr, errorLen);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
    }

    internal static StorageProgress ToModel(this NativeOperationSnapshot snapshot) =>
        new(
            snapshot.HasTotalBytes == 0 ? null : snapshot.TotalBytes,
            snapshot.BytesTransferred,
            snapshot.BytesPerSecond);

    internal static LogLevel ToLogLevel(this NativeLogRecordDto record) =>
        record.Level.ToLogLevel();

    internal static LogLevel ToLogLevel(this string level) =>
        level.ToUpperInvariant() switch
        {
            "TRACE" => LogLevel.Trace,
            "DEBUG" => LogLevel.Debug,
            "INFO" => LogLevel.Information,
            "WARN" => LogLevel.Warning,
            "ERROR" => LogLevel.Error,
            _ => LogLevel.Information,
        };
}
