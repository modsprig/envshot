using System.Text;
using System.Text.Json;

namespace EnvShot;

/// A non-zero status from the native library.
public sealed class EnvShotException : Exception
{
    /// The native status code (1 internal, 2 invalid input, 3 operation error).
    public int Status { get; }
    /// The stable error code from the error envelope, when present.
    public string? Code { get; }

    internal EnvShotException(int status, string? code, string message)
        : base($"envshot call failed (status {status}, code {code ?? "none"}): {message}")
    {
        Status = status;
        Code = code;
    }
}

/// Safe wrapper over the envshot_ffi C ABI. No capture/diff/fingerprint
/// semantics live here: the native library is the single source of truth.
public static class EnvShotClient
{
    /// Resolves envshot_ffi from the assembly directory (the native library
    /// ships next to the managed assembly).
    [System.Runtime.CompilerServices.ModuleInitializer]
    internal static void RegisterNativeResolver() =>
        System.Runtime.InteropServices.NativeLibrary.SetDllImportResolver(
            typeof(EnvShotClient).Assembly, ResolveNative);

    private static nint ResolveNative(
        string libraryName,
        System.Reflection.Assembly assembly,
        System.Runtime.InteropServices.DllImportSearchPath? searchPath)
    {
        if (libraryName != "envshot_ffi")
        {
            return 0;
        }
        foreach (var candidate in new[]
                 {
                     "envshot_ffi.dll",
                     "libenvshot_ffi.so",
                     "libenvshot_ffi.dylib",
                 })
        {
            var path = System.IO.Path.Combine(AppContext.BaseDirectory, candidate);
            if (System.Runtime.InteropServices.NativeLibrary.TryLoad(path, out var handle))
            {
                return handle;
            }
        }
        return 0;
    }

    /// The ABI version of the loaded native library.
    public static uint AbiVersion => EnvShotNative.EnvShotAbiVersion();

    /// Captures a snapshot. Optional keys: profile ("minimal"|"standard"|
    /// "empty_test"), privacy ("standard"|"strict"|"off"). Defaults:
    /// standard/standard.
    public static JsonDocument Capture(string? profile = null, string? privacy = null)
    {
        var config = BuildConfig(("profile", profile), ("privacy", privacy));
        unsafe
        {
            fixed (byte* configPtr = config.AsSpanOrNull())
            {
                var status = EnvShotNative.EnvShotCaptureJson(
                    (nint)configPtr, (nuint)config.Length, out var bufferPtr);
                return Finish(status, bufferPtr);
            }
        }
    }

    /// Diffs two snapshot documents under the given policy
    /// ("compatibility"|"diagnostic"; default compatibility).
    public static JsonDocument Diff(string beforeJson, string afterJson, string? policy = null)
    {
        var config = BuildConfig(("policy", policy));
        var before = Encoding.UTF8.GetBytes(beforeJson);
        var after = Encoding.UTF8.GetBytes(afterJson);
        unsafe
        {
            fixed (byte* configPtr = config.AsSpanOrNull(),
                   beforePtr = before, afterPtr = after)
            {
                var status = EnvShotNative.EnvShotDiffJson(
                    (nint)beforePtr, (nuint)before.Length,
                    (nint)afterPtr, (nuint)after.Length,
                    (nint)configPtr, (nuint)config.Length,
                    out var bufferPtr);
                return Finish(status, bufferPtr);
            }
        }
    }

    /// Computes a fingerprint (default profile repro-v1). The result document
    /// carries the value and the projection (the explain surface).
    public static JsonDocument Fingerprint(string snapshotJson, string? profile = null)
    {
        var config = BuildConfig(("profile", profile));
        var snapshot = Encoding.UTF8.GetBytes(snapshotJson);
        unsafe
        {
            fixed (byte* configPtr = config.AsSpanOrNull(), snapshotPtr = snapshot)
            {
                var status = EnvShotNative.EnvShotFingerprintJson(
                    (nint)snapshotPtr, (nuint)snapshot.Length,
                    (nint)configPtr, (nuint)config.Length,
                    out var bufferPtr);
                return Finish(status, bufferPtr);
            }
        }
    }

    private static byte[] BuildConfig(params (string Key, string? Value)[] fields)
    {
        var dict = new Dictionary<string, string>();
        foreach (var (key, value) in fields)
        {
            if (value is not null)
            {
                dict[key] = value;
            }
        }
        return dict.Count == 0
            ? Array.Empty<byte>()
            : JsonSerializer.SerializeToUtf8Bytes(dict);
    }

    private static Span<byte> AsSpanOrNull(this byte[] bytes) =>
        bytes.Length == 0 ? default : bytes.AsSpan();

    /// Reads the returned buffer into managed memory, frees it, and surfaces
    /// non-zero statuses as typed exceptions.
    private static JsonDocument Finish(int status, nint buffer)
    {
        try
        {
            unsafe
            {
                var native = System.Runtime.InteropServices.Marshal.PtrToStructure<
                    EnvShotBufferNative>(buffer);
                // Copy into managed memory before freeing the native buffer;
                // the document stays valid after the free.
                var span = new ReadOnlySpan<byte>(
                    (void*)native.Ptr, checked((int)native.Len));
                var document = JsonDocument.Parse(span.ToArray());
                if (status == 0)
                {
                    return document;
                }
                var code = document.RootElement
                    .GetProperty("error").GetProperty("code").GetString();
                var message = document.RootElement
                    .GetProperty("error").GetProperty("message").GetString();
                throw new EnvShotException(status, code, message ?? string.Empty);
            }
        }
        finally
        {
            EnvShotNative.EnvShotFreeBuffer(buffer);
        }
    }
}
