using System.Text.Json;
using EnvShot;
using Xunit;

public class EnvShotTests
{
    private const string GoldenFingerprint =
        "repro-v1:sha256:4119291c8cf461350058f0cedf1de7ff7d5fdb75d5ee6bc0f3045599b32608a7";

    private static readonly string CompleteSnapshot = """
        {"schema":{"name":"envshot.snapshot","major":0,"minor":1},"producer":{"name":"envshot","version":"0.0.0"},"capture":{"profile":"standard","privacy":"standard"},"sections":{"architecture":{"status":"available","data":{"process":"x64","os":"x64"},"provenance":[],"issues":[]},"cpu":{"status":"available","data":{"model":"Fixture CPU","logical_count":8,"physical_count":4,"features":["avx2","sse4_2"]},"provenance":[{"collector_id":"cpu.features","revision":1,"source_kind":"system_file","notes":null}],"issues":[]},"locale":{"status":"available","data":{"languages":["en-US"],"timezone":"Etc/UTC"},"provenance":[],"issues":[]},"memory":{"status":"available","data":{"installed_bytes":17179869184},"provenance":[],"issues":[]},"os":{"status":"available","data":{"family":"windows","version":{"raw":"10.0.26100","normalized":{"major":10,"minor":0,"patch":26100}},"build":{"raw":"26100.1742","normalized":{"major":26100,"minor":1742,"patch":null}}},"provenance":[{"collector_id":"os.version","revision":1,"source_kind":"native_api","notes":null}],"issues":[]},"virtualization":{"status":"available","data":{"category":"physical"},"provenance":[],"issues":[]}}}
        """;

    /// The C# wrapper must observe the same ABI version contract.
    [Fact]
    public void AbiVersionIsOne() => Assert.Equal(1U, EnvShotClient.AbiVersion);

    /// Scenario 08.9: Rust/C# golden vectors match (fingerprint equivalence).
    [Fact]
    public void FingerprintMatchesTheRustGoldenVector()
    {
        using var document = EnvShotClient.Fingerprint(CompleteSnapshot);
        Assert.Equal(GoldenFingerprint,
            document.RootElement.GetProperty("fingerprint").GetString());
        Assert.Equal("windows",
            document.RootElement.GetProperty("projection")
                .GetProperty("facts").GetProperty("os.family").GetString());
    }

    /// Capture through the wrapper records the standard privacy default.
    [Fact]
    public void CaptureDefaultsToStandardPrivacy()
    {
        using var document = EnvShotClient.Capture();
        Assert.Equal("standard",
            document.RootElement.GetProperty("capture").GetProperty("privacy").GetString());
        Assert.True(document.RootElement.GetProperty("sections")
            .EnumerateObject().Any(), "a real host has sections");
    }

    /// Diff through the wrapper produces the versioned envelope.
    [Fact]
    public void DiffProducesTheVersionedEnvelope()
    {
        using var document = EnvShotClient.Diff(CompleteSnapshot, CompleteSnapshot);
        Assert.Equal("envshot.diff",
            document.RootElement.GetProperty("diff_schema").GetProperty("name").GetString());
        Assert.Equal(JsonValueKind.Array,
            document.RootElement.GetProperty("changes").ValueKind);
    }

    /// Invalid config input surfaces as a typed exception with status 2.
    [Fact]
    public void InvalidInputThrowsWithStatus()
    {
        var exception = Assert.Throws<EnvShotException>(
            () => EnvShotClient.Fingerprint("{ not json"));
        Assert.Equal(3, exception.Status);
        Assert.Equal("snapshot.read.malformed", exception.Code);
    }

    /// Repeated capture/free cycles must not corrupt the native allocator.
    [Fact]
    public void RepeatedCallsRemainStable()
    {
        for (var i = 0; i < 32; i++)
        {
            using var document = EnvShotClient.Capture();
            Assert.Equal("standard",
                document.RootElement.GetProperty("capture").GetProperty("privacy").GetString());
        }
    }
}
