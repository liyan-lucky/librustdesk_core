# librustdesk_core

RustDesk HarmonyOS native core static library builder. Builds `librustdesk_core.a` from RustDesk 1.4.7 upstream source with OHOS cross-compilation, and generates the complete C++ NAPI bridge layer for HarmonyOS ArkTS applications.

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
| `undefined_symbols.txt` | Undefined symbol list for link debugging |

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

The Windows script prepares target static dependencies before Cargo runs. On a cold runner it builds `libsodium`, downloads and builds `libvpx` `1.15.2`, downloads and builds `libyuv` revision `0faf8dd0e004520a61a603a4d2996d5ecc80dc3f`, and installs them under `VCPKG_INSTALLED_ROOT\arm64-linux`. The libvpx/libyuv C++ builds use `-nostdinc++` plus the OpenHarmony SDK libc++ include directory, because GitHub Actions runners may not discover `<cstdint>` from `--sysroot` alone.

### Linux

```bash
./scripts/build_native_bridge.sh aarch64-unknown-linux-ohos release
```

### CI/CD

GitHub Actions workflow: `.github/workflows/build-core.yml`
- Runs on `windows-2022`
- Rust toolchain: 1.88.0
- Builds with Cargo `release` profile from `native_rust_core/Cargo.toml`
- Rejects suspicious release assets outside `100,000,000` to `250,000,000` bytes
- Output: `librustdesk_core.a` uploaded as release asset

### Output

- Static library: `native_rust_core/target/aarch64-unknown-linux-ohos/release/librustdesk_harmony_bridge.a`
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
- Harmony incoming/controlled-side screen sharing is not available yet because the desktop server thread and Harmony screen-capture pipeline are not wired on this target.
- `main_start_service(true)` must return `incomingReady=false` with a clear error while that pipeline is missing. Do not mark incoming ready just because rendezvous/options were refreshed; that makes remote clients wait forever for a video stream that cannot exist.
