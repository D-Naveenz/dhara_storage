using System.Runtime.InteropServices;

namespace Dhara.Storage.Interop.Native;

internal enum NativeStatus
{
    Ok = 0,
    Error = 1,
    InvalidArgument = 2,
    Panic = 3,
}

internal enum NativeOperationState : byte
{
    Running = 0,
    Completed = 1,
    Failed = 2,
    Cancelled = 3,
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeOperationSnapshot
{
    internal NativeOperationState State;
    internal byte HasTotalBytes;
    internal ulong TotalBytes;
    internal ulong BytesTransferred;
    internal double BytesPerSecond;
}
