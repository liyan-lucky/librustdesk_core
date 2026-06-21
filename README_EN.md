# librustdesk_core

RustDesk HarmonyOS native core static library builder. Builds `librustdesk_core.a` from RustDesk 1.4.7 upstream source with OHOS cross-compilation, and generates the complete C++ NAPI bridge layer for HarmonyOS ArkTS applications.

> 2026-06-21 23:38 integration candidate: arm64 archive `131,091,732` bytes, SHA256 `E4614BAE4EDB54F2C0A2CFECE96A2E99D558B6900693B2B3A9B08B8F3DCD5D5D`; x86_64 archive `130,090,572` bytes, SHA256 `DB0283F44EA5E5D09A23D1756929B171F28FF2A602D595941902A18ECE5F17DD`. Both are local 2026-06-21 builds from the same source baseline, embedded in the final App HAP and verified by the package audit, 100-round audit and device cold start. Huawei controlled-side input injection is intentionally shelved as unsupported.

[中文](README.md)

## Architecture

```
ArkTS UI (11_Rustdesk_harmonyos)
    -> NAPI
librustdesk_bridge.so
    -> C++ bridge loader (cpp/)
    -> Rust C ABI (native_rust_core/)
librustdesk_core.a
    -> rustdesk_harmony_bridge
    -> RustDesk official session/core (rustdesk-master/)
RustDesk Server / Peer
```

## Structure

| Directory | Description |
|-----------|-------------|
| `native_rust_core/` | Rust bridge layer (bridge_api.rs, bridge_state.rs, lib.rs) |
| `rustdesk-master/` | Upstream RustDesk source (1.4.7) with OHOS patches |
| `patches/` | OHOS-specific crate patches (machine-uid) |
| `rdev-fork/` | rdev input library fork with OHOS support |
| `cpp/` | C++ NAPI bridge layer (abi.h, loader.cpp, CMakeLists.txt) |
| `scripts/` | Build scripts and code generators |

## Key Files

### Rust Bridge Layer (`native_rust_core/`)

| File | Description |
|------|-------------|
| `src/bridge_api.rs` | C FFI exports (~2872 lines), all `rustdesk_bridge_*` functions |
| `src/bridge_state.rs` | Bridge state snapshot management (BridgeSnapshot, event queue) |
| `src/lib.rs` | Crate entry point |
| `Cargo.toml` | `crate-type = ["staticlib"]`, depends on rustdesk 1.4.7 |
| `build.rs` | Adds `-Wl,-z,notext` for OHOS target |

### C++ NAPI Bridge Layer (`cpp/`)

| File | Description |
|------|-------------|
| `rustdesk_bridge_abi.h` | C ABI header declaring all `rustdesk_bridge_*` functions |
| `rustdesk_bridge_loader.cpp` | NAPI module loader, wraps C ABI as NAPI exports |
| `ohos_stubs.cpp` | OHOS platform stubs (xcb, OH_TimeService, qsort_r) |
| `CMakeLists.txt` | Links `librustdesk_core.a` into `librustdesk_bridge.so` |
| `types/librustdesk_bridge/index.d.ts` | TypeScript type declarations for NAPI module |

### Code Generation Scripts (`scripts/`)

| Script | Description |
|--------|-------------|
| `generate_bridge_api.js` | Generate bridge_api.rs from core.rs |
| `generate_cpp_bridge.js` | Generate ABI header and NAPI loader from core.rs |
| `generate_ts_bridge.js` | Generate TS type declarations from core.rs |
| `regenerate_all.js` | One-click regenerate all bridge code |
| `dedup_abi.js` | Deduplicate ABI header declarations |
| `dedup_loader.js` | Deduplicate NAPI registrations |
| `dedup_loader_funcs.js` | Deduplicate NAPI function definitions |
| `rename_mapping.js` | OHOS name to official wire_ name mapping |
| `build_native_bridge.ps1` | Windows cross-compilation build script |
| `build_native_bridge.sh` | Linux/macOS build script |

## Build

### Windows (Primary)

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build_native_bridge.ps1
```

The Windows script prepares target static dependencies before Cargo runs. On a cold runner it builds `libsodium`, downloads and builds `libvpx` `1.15.2`, downloads and builds `libyuv` revision `0faf8dd0e004520a61a603a4d2996d5ecc80dc3f`, and installs them under `VCPKG_INSTALLED_ROOT\arm64-linux`. For `libvpx`, build only the `libvpx.a` make target and manually install the public headers; do not run the default `make && make install` path, because it also builds the unused `libvpxrc.a` from C++ RTC rate-control sources (`vp9/ratectrl_rtc.cc`, `vp8/vp8_ratectrl_rtc.cc`). Minimal online SDK zips may not contain compatible libc++ headers, and MSYS2 libc++ is not a safe fallback for the OHOS clang bundled with the SDK. `libyuv` may use the SDK libc++ include directory when it exists, but the build must not depend on MSYS2 libc++ headers.

### Linux

```bash
./scripts/build_native_bridge.sh aarch64-unknown-linux-ohos release
```

### CI/CD

**Windows online build**: `.github/workflows/build-core-windows.yml`
- Runs on `windows-2022`
- Rust toolchain: 1.88.0
- Builds with Cargo `release` profile from `native_rust_core/Cargo.toml`
- Rejects suspicious release assets outside `100,000,000` to `250,000,000` bytes
- Output: `librustdesk_core.a` uploaded as release asset

**Linux online build**: `.github/workflows/build-core-linux.yml`
- Runs on `ubuntu-22.04`
- Rust toolchain: 1.88.0
- Manual trigger only (`workflow_dispatch`), no auto-trigger
- Uses the same dependencies and compilation logic as the Windows build
- Requires repository secret `OHOS_SDK_LINUX_ZIP_URL` (Linux OHOS Native SDK download URL)

### Output

- Standard local static library: `%VSCODE_ROOT%\99_Temp\librustdesk_core\cargo_target\<target-triple>\release\librustdesk_harmony_bridge.a`
- Repository-local `native_rust_core/target/` is regenerable cache and is no longer retained after the 2026-06-21 cleanup.
- Rename to `librustdesk_core.a` when copying to HAP project

## Usage in HAP Project

1. Download `librustdesk_core.a` from [GitHub Releases](https://github.com/liyan-lucky/librustdesk_core/releases)
2. Copy to `11_Rustdesk_harmonyos/entry/src/main/libs/arm64/librustdesk_core.a`
3. Copy `cpp/` files to `11_Rustdesk_harmonyos/entry/src/main/cpp/` (if bridge layer updated)
4. Copy `cpp/types/` to `11_Rustdesk_harmonyos/entry/src/main/cpp/types/` (if TS declarations updated)
5. Build HAP: `scripts\build_hap.bat`

## Function Name Mapping

OHOS uses `rustdesk_bridge_*` prefix for all C FFI functions. Some names differ from official `wire_*` names:

| OHOS Name | Official Name | Notes |
|-----------|---------------|-------|
| connect_to_peer | session_start | NAPI preserves old name, calls new C function |
| set_incoming_service_enabled | main_start_service | NAPI preserves old name |
| session_alternative_codecs | session_get_alternative_codecs | Renamed to match official |
| main_use_texture_render | main_get_use_texture_render | Renamed to match official |

See `scripts/rename_mapping.js` for complete mapping.

## Upstream Compatibility

- Current version: RustDesk 1.4.7
- OHOS target: `aarch64-unknown-linux-ohos`
- Key OHOS adaptations:
  - `cfg(target_env = "ohos")` excludes desktop Linux dependencies
  - `scrap` without wayland/gtk/dbus features
  - `arboard` without wayland-data-control feature
  - Independent `rendezvous_mediator_ohos.rs` for LAN discovery
  - `harmony_bridge/core.rs` as session entry point (not flutter_ffi.rs)

## Current Video and Incoming Service Status

- Outgoing remote-control sessions use the real RustDesk session path and publish video through `on_rgba -> publish_real_video_frame -> video-frame`.
- OHOS outgoing viewer video decode uses software VP8/VP9 through `libvpx` plus YUV-to-RGBA conversion through `libyuv`. `codec_ohos.rs` must not advertise VP9 support unless `handle_video_frame()` can decode frames and call `GoogleImage::to()`.
- Keep libvpx VP8/VP9 encoders enabled unless `scrap` bindings are redesigned. `scrap/src/bindings/vpx_ffi.h` includes `vp8cx.h` and `vpx_encoder.h`, and `common/vpxcodec.rs` references encoder APIs even when the current OHOS user flow is viewer-side decode. Skip only `libvpxrc.a`; do not disable the encoders that produce the public C API used by `scrap`.
- Harmony incoming/controlled-side screen sharing is not available yet because the desktop server thread and Harmony screen-capture pipeline are not wired on this target.
- `main_start_service(true)` must return `incomingReady=false` with a clear error while that pipeline is missing. Do not mark incoming ready just because rendezvous/options were refreshed; that makes remote clients wait forever for a video stream that cannot exist.
