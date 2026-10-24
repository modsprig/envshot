# Unsafe inventory

Workspace policy: `unsafe_code = "forbid"` everywhere except two crates with
documented, scoped opt-outs (`envshot-capture`, `envshot-ffi`). The pure
crates (`envshot-model`, `envshot-core`) contain no unsafe at all.

## `envshot-capture` (Windows platform adapters)

| Site | Why | Safety invariant | Caller guarantee | Tests |
|---|---|---|---|---|
| `registry_reader.rs::read_raw` (`RegGetValueW`) | No safe std/windows-wrapper alternative for a single documented value read | NUL-terminated UTF-16 buffers outlive the call; buffer/size are valid out-parameters; RegGetValueW opens no handle needing release | `key`/`name` are `&str` (valid for the call) | real-host reads; unknown value → None; ctypes + FFI cross-checks |
| `architecture.rs::wow64_machines` (`IsWow64Process2`) | Only API that cannot lie about the native machine under emulation | Two valid machine-code out-parameters; pseudo-handle from GetCurrentProcess needs no close | No external inputs | real-host x64/x64 invariant |
| `gpu.rs::enumerate_adapters` (`CreateDXGIFactory1`, `EnumAdapters1`, `GetDesc1`, `CheckInterfaceSupport`) | DXGI is the documented GPU identity source | COM reference counting handled by windows-crate RAII wrappers; Description buffer read via documented UTF-16 contract | No external inputs | real-host enumeration; mapping tests |

## `envshot-ffi` (C ABI boundary)

| Site | Why | Safety invariant | Caller guarantee | Tests |
|---|---|---|---|---|
| `input_string` | Reads caller-provided byte buffers | `len` bytes readable at `ptr` for the call duration | ABI contract (documented on each export) | invalid-UTF-8 + null tests (08.A) |
| `envshot_free_buffer` | Reconstructs the allocation for exact deallocation | Buffer must come from this library, freed at most once, or null | ABI contract | repeated-call/free stability tests (64 cycles) |
| `#[unsafe(no_mangle)]` exports | Edition-2024 requirement for exported symbols | Export names are unique; signatures are ABI-stable | n/a | abi.rs suite |

## Review conclusion

No unsafe exists outside the two sanctioned boundaries; every site has a
safety comment at the use site and tests exercising the wrapped behavior.
Safe-wrapper alternatives were evaluated (windows-crate RAII covers COM
lifetimes); no additional wrapper is warranted for the remaining calls.
