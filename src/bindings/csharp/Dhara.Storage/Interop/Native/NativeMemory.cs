using System.Runtime.InteropServices;
using System.Text;

namespace Dhara.Storage.Interop.Native;

internal static partial class NativeMemory
{
    internal const string LibraryName = "dharastorage";

    [LibraryImport(LibraryName)]
    internal static partial void dhara_string_free(nint ptr, nuint len);

    [LibraryImport(LibraryName)]
    internal static partial void dhara_bytes_free(nint ptr, nuint len);

    internal static string ReadUtf8AndFree(nint ptr, nuint len)
    {
        if (ptr == 0 || len == 0)
        {
            return string.Empty;
        }

        var bytes = ReadBytes(ptr, len);
        dhara_string_free(ptr, len);
        return Encoding.UTF8.GetString(bytes);
    }

    internal static byte[] ReadBytesAndFree(nint ptr, nuint len)
    {
        if (ptr == 0 || len == 0)
        {
            return [];
        }

        var bytes = ReadBytes(ptr, len);
        dhara_bytes_free(ptr, len);
        return bytes;
    }

    internal static string ReadUtf8(nint ptr, nuint len)
    {
        if (ptr == 0 || len == 0)
        {
            return string.Empty;
        }

        return Encoding.UTF8.GetString(ReadBytes(ptr, len));
    }

    private static byte[] ReadBytes(nint ptr, nuint len)
    {
        var bytes = new byte[(int)len];
        Marshal.Copy(ptr, bytes, 0, bytes.Length);
        return bytes;
    }
}
