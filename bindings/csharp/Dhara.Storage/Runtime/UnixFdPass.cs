using System.Net.Sockets;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace Dhara.Storage.Runtime;

/// <summary>
/// Receives file descriptors passed with <c>SCM_RIGHTS</c> over the daemon FD-pass socket.
/// </summary>
internal static partial class UnixFdPass
{
    /// <summary>Connects to the daemon FD-pass endpoint.</summary>
    internal static Socket Connect(string endpointPath)
    {
        var socket = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
        socket.Connect(new UnixDomainSocketEndPoint(endpointPath));
        return socket;
    }

    /// <summary>Receives the next passed file descriptor from the daemon.</summary>
    internal static SafeFileHandle Receive(Socket socket)
    {
        var fd = ReceiveRaw(socket.Handle);
        return new SafeFileHandle((nint)fd, ownsHandle: true);
    }

    private static int ReceiveRaw(nint socketHandle)
    {
        var control = new byte[64];
        var payload = new byte[1];
        var ioVec = new IoVec
        {
            IovBase = Marshal.AllocHGlobal(payload.Length),
            IovLen = (nuint)payload.Length,
        };
        Marshal.Copy(payload, 0, ioVec.IovBase, payload.Length);

        var message = new Msghdr
        {
            MsgIov = Marshal.AllocHGlobal(Marshal.SizeOf<IoVec>()),
            MsgIovlen = 1,
            MsgControl = Marshal.AllocHGlobal(control.Length),
            MsgControllen = (nuint)control.Length,
        };
        Marshal.StructureToPtr(ioVec, message.MsgIov, false);

        try
        {
            var received = recvmsg((int)socketHandle, ref message, 0);
            if (received < 0)
            {
                throw new InvalidOperationException(
                    $"recvmsg failed with errno {Marshal.GetLastPInvokeError()}");
            }

            var cmsg = CmsgFirstHdr(ref message);
            while (cmsg != nint.Zero)
            {
                var header = Marshal.PtrToStructure<Cmsghdr>(cmsg)!;
                if (header.CmsgLevel == SolSocket && header.CmsgType == ScmRights)
                {
                    var data = cmsg + Marshal.SizeOf<Cmsghdr>();
                    var fds = new int[1];
                    Marshal.Copy(data, fds, 0, 1);
                    if (fds[0] < 0)
                    {
                        throw new InvalidOperationException("SCM_RIGHTS returned an invalid file descriptor.");
                    }

                    return fds[0];
                }

                cmsg = CmsgNxtHdr(ref message, cmsg);
            }

            throw new InvalidOperationException("recvmsg did not include SCM_RIGHTS ancillary data.");
        }
        finally
        {
            if (ioVec.IovBase != nint.Zero)
            {
                Marshal.FreeHGlobal(ioVec.IovBase);
            }

            if (message.MsgIov != nint.Zero)
            {
                Marshal.FreeHGlobal(message.MsgIov);
            }

            if (message.MsgControl != nint.Zero)
            {
                Marshal.FreeHGlobal(message.MsgControl);
            }
        }
    }

    private static nint CmsgFirstHdr(ref Msghdr message) =>
        message.MsgControllen >= (nuint)Marshal.SizeOf<Cmsghdr>()
            ? message.MsgControl
            : nint.Zero;

    private static nint CmsgNxtHdr(ref Msghdr message, nint cmsg)
    {
        var header = Marshal.PtrToStructure<Cmsghdr>(cmsg);
        var next = cmsg + (nint)header.CmsgLen;
        var end = message.MsgControl + (nint)message.MsgControllen;
        if (next + Marshal.SizeOf<Cmsghdr>() > end)
        {
            return nint.Zero;
        }

        return next;
    }

    private static nuint CmsgAlign(nuint length)
    {
        var wordSize = (nuint)IntPtr.Size;
        return (length + wordSize - 1) & ~(wordSize - 1);
    }

    private const int SolSocket = 1;
    private const int ScmRights = 1;

    [LibraryImport("libc", SetLastError = true)]
    private static partial nint recvmsg(int socket, ref Msghdr message, int flags);

    [StructLayout(LayoutKind.Sequential)]
    private struct IoVec
    {
        public nint IovBase;
        public nuint IovLen;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Msghdr
    {
        public nint MsgName;
        public nuint MsgNamelen;
        public nint MsgIov;
        public nuint MsgIovlen;
        public nint MsgControl;
        public nuint MsgControllen;
        public int MsgFlags;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Cmsghdr
    {
        public nuint CmsgLen;
        public int CmsgLevel;
        public int CmsgType;
    }
}
