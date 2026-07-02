namespace Dhara.Storage.Models.Information;

/// <summary>
/// Helpers for turning <see cref="ShellIcon"/> RGBA buffers into common GUI image forms.
/// </summary>
public static class ShellIconExtensions
{
    /// <summary>
    /// Copies RGBA pixels into a new array suitable for texture upload or further encoding.
    /// </summary>
    public static byte[] ToRgbaArray(this ShellIcon icon) => icon.RgbaPixels.ToArray();
}
