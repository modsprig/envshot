namespace EnvShot;

/// P/Invoke surface of envshot_ffi. Statuses: 0 ok, 1 internal, 2 invalid
/// input, 3 operation error. All buffers are owned by the native library and
/// freed only through EnvShotFreeBuffer.
internal static unsafe partial class EnvShotNative
{
    internal const uint AbiVersion = 1;

    [System.Runtime.InteropServices.LibraryImport("envshot_ffi", EntryPoint = "envshot_abi_version")]
    [return: System.Runtime.InteropServices.MarshalAs(
        System.Runtime.InteropServices.UnmanagedType.U4)]
    internal static partial uint EnvShotAbiVersion();

    [System.Runtime.InteropServices.LibraryImport("envshot_ffi", EntryPoint = "envshot_capture_json")]
    internal static partial int EnvShotCaptureJson(
        nint configPtr,
        nuint configLen,
        out nint outBuffer);

    [System.Runtime.InteropServices.LibraryImport("envshot_ffi", EntryPoint = "envshot_diff_json")]
    internal static partial int EnvShotDiffJson(
        nint aPtr, nuint aLen,
        nint bPtr, nuint bLen,
        nint policyPtr, nuint policyLen,
        out nint outBuffer);

    [System.Runtime.InteropServices.LibraryImport("envshot_ffi", EntryPoint = "envshot_fingerprint_json")]
    internal static partial int EnvShotFingerprintJson(
        nint snapshotPtr, nuint snapshotLen,
        nint profilePtr, nuint profileLen,
        out nint outBuffer);

    [System.Runtime.InteropServices.LibraryImport("envshot_ffi", EntryPoint = "envshot_free_buffer")]
    internal static partial void EnvShotFreeBuffer(nint buffer);
}

[System.Runtime.InteropServices.StructLayout(
    System.Runtime.InteropServices.LayoutKind.Sequential)]
internal struct EnvShotBufferNative
{
    public nint Ptr;
    public nuint Len;
    public nuint Capacity;
}
