# Typed C-Compatible ABI Between Rust and C#

Use typed C-compatible ABI structs for hot native-to-managed structured data. JSON is useful for diagnostics, logs, and cold-path error payloads, but it is too expensive for frequent query results because it adds serialization, allocation, UTF-8 string transfer, parsing, and DTO reconstruction on every call.

The Dhara native ABI should expose stable C-shaped memory, not Rust-shaped memory. Rust owns the native result allocation, C# reads it immediately into managed models, then C# calls the matching Rust free function.

## Core Rule

Rust structs that cross the ABI must use `#[repr(C)]`. C# mirrors must use `[StructLayout(LayoutKind.Sequential)]`.

Only put C-compatible fields inside ABI structs:

- integers: `u8`, `u32`, `u64`, `usize`
- floating point: `f64`
- pointers: `*const T`, `*mut T`
- pointer-sized C# fields: `nint`, `nuint`
- fixed-layout nested `#[repr(C)]` structs

Do not expose these across the ABI:

- Rust `String`, `&str`, `Vec<T>`, `PathBuf`, `Option<T>`, `bool`, or Rust enums
- C# `string`, arrays, `bool`, or reference types inside `[StructLayout]` result structs
- borrowed pointers into temporary Rust stack values or short-lived Rust objects

## Data Shapes

Use explicit ABI shapes for dynamic and optional data.

```rust
#[repr(C)]
pub struct NativeUtf8 {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
pub struct NativeOptionalUtf8 {
    pub has_value: u8,
    pub value: NativeUtf8,
}

#[repr(C)]
pub struct NativeEntryList {
    pub entries_ptr: *const NativeEntry,
    pub entries_len: usize,
}
```

```csharp
[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeUtf8
{
    internal readonly nint Ptr;
    internal readonly nuint Len;
}

[StructLayout(LayoutKind.Sequential)]
internal readonly struct NativeOptionalUtf8
{
    internal readonly byte HasValue;
    internal readonly NativeUtf8 Value;
}
```

Represent values consistently:

- strings: UTF-8 `ptr + len`, not null-terminated strings inside result structs
- arrays: element `ptr + len`
- optional values: `has_value: u8` plus the value field
- booleans: `u8`, where `0` is false and non-zero is true
- enums: `u32` discriminants with documented numeric values
- timestamps: fixed integer values, such as Unix milliseconds in `u64`

## Ownership Pattern

Prefer Rust-owned result handles for complex payloads.

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhara_list_entries(
    path: *const c_char,
    recursive: u8,
    out_entries: *mut *mut NativeStorageEntryList,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> DharaStatus;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dhara_storage_entry_list_free(entries: *mut NativeStorageEntryList);
```

The Rust owner object should keep all backing buffers alive for as long as the top-level ABI pointer is alive. A common pattern is:

- ABI struct as the first field of a private owner struct
- boxed strings stored as `Vec<Box<[u8]>>`
- arrays stored as `Box<[NativeItem]>`
- nested owned result data stored in the same owner when possible

C# must:

- call the native function
- throw or translate error payloads before reading success data
- copy/read the native struct and nested slices immediately
- convert into public managed models
- call the matching free function in `finally`

```csharp
var status = NativeQueries.dhara_list_entries(path, recursive, out var entries, out var errorPtr, out var errorLen);
NativeHelpers.ThrowIfFailed(status, errorPtr, errorLen);
try
{
    return NativeTyped.ToStorageEntries(entries);
}
finally
{
    NativeQueries.dhara_storage_entry_list_free(entries);
}
```

## C# Reading Rules

Use one central marshalling helper for pointer reads and conversions.

```csharp
private static string ToString(NativeUtf8 value)
{
    if (value.Ptr == 0 || value.Len == 0)
    {
        return string.Empty;
    }

    return Encoding.UTF8.GetString(new ReadOnlySpan<byte>((void*)value.Ptr, checked((int)value.Len)));
}
```

Rules for C# helpers:

- validate null top-level pointers before `Marshal.PtrToStructure` or unsafe reads
- convert `nuint` to `int` with `checked` when building spans or arrays
- treat native slices as borrowed and short-lived
- never store native pointers in public managed objects
- never free native memory with C# or the GC; always call the Rust free function

## Rust Safety Rules

Rules for Rust builders:

- initialize every ABI field explicitly
- use null pointer plus zero length for empty strings and empty arrays
- keep backing storage in the owner object, not in local variables
- do not return pointers to stack memory
- do not expose Rust allocator-owned memory without a Rust free function
- validate all incoming pointers before dereferencing
- reset output pointers on entry so failed calls do not expose stale data

Rules for exported functions:

- use `extern "C"` and `#[unsafe(no_mangle)]`
- return a status code, not a Rust `Result`
- write success payloads through out-pointers
- keep error payloads separate from success payloads
- keep error JSON acceptable for cold paths and diagnostics

## Layout Verification

Add layout tests for important ABI structs, especially those with pointers, `usize`, nested structs, or alignment-sensitive fields.

Rust tests should check at least:

- `size_of::<T>()`
- `align_of::<T>()`
- selected field offset assumptions when layout is non-trivial

C# tests should exercise every typed path and compare the public managed model output against expected behavior. Native package verification should prove the consuming app can load the native DLL from the packaged runtime asset.

## When JSON Is Still Acceptable

Do not force typed ABI for every byte of data. JSON is still reasonable for:

- error payloads
- logging records
- diagnostic-only metadata
- rare payloads whose shape is intentionally flexible

Use typed ABI for hot structured result paths such as listings, file information, directory information, analysis reports, and watch events.

## Checklist For New Typed ABI Results

- Add a `#[repr(C)]` Rust struct using only ABI-safe fields.
- Add a matching C# `[StructLayout(LayoutKind.Sequential)]` struct.
- Use UTF-8 `ptr + len` for strings.
- Use `ptr + len` for arrays.
- Use `u8` presence flags for optional values.
- Use `u32` discriminants for enums.
- Return a Rust-owned top-level result pointer.
- Add a matching Rust free function.
- Convert immediately into public managed models.
- Add Rust layout tests and C# behavior tests.
- Keep the public .NET API unchanged unless the product API itself needs to change.

## Related docs

- [dharastorage README][readme-dharastorage] — exported C entry points and logger bridge
- [Dhara.Storage README][readme-nuget] — managed consumer API
- [CI/CD pipelines][ci-cd] — native package verification on Windows CI
- [Docs index][docs-index]

[readme-dharastorage]: ../src/bindings/dharastorage-ffi/README.md
[readme-nuget]: ../src/bindings/csharp/Dhara.Storage/README.md
[ci-cd]: ci-cd-pipelines.md
[docs-index]: README.md
