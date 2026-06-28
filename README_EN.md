# librustdesk_core

> Disclaimer: This is an unofficial third-party HarmonyOS / OpenHarmony adaptation project. It is not affiliated with, endorsed by, sponsored by, or officially maintained by the upstream project. Upstream project names and related marks are used only to identify source origin and compatibility targets.
>
> License and source notice: this repository contains HarmonyOS adaptation code, bridge code, build scripts, and upstream-derived source. When using, modifying, distributing, or redistributing this repository or its build outputs, review and comply with the upstream license and third-party dependency licenses. See `NOTICE` and `docs/THIRD_PARTY_NOTICES.md`.

RustDesk HarmonyOS native core static library builder. It builds `librustdesk_core.a` from the RustDesk 1.4.7 upstream source through OHOS cross-compilation and provides a C++ NAPI bridge layer for HarmonyOS ArkTS applications.

[中文](README.md)

## Current status

- Release tags use the unified `core-001`, `core-002`, `core-003` format.
- Windows and Linux builds share the same release-number sequence.
- A release number is reserved at build start; failed builds keep the reserved tag.
- A GitHub Release and release assets are created only after both arm64 and x86_64 artifacts are generated and validated.
- Release notes are updated without changing the tag or release name.
- Linux can be triggered automatically after main-branch updates or manually. Windows remains manually triggered.

## Architecture

```
ArkTS UI (11_Rustdesk_harmonyos)
    -> NAPI
librustdesk_bridge.so
    -> C++ bridge loader (cpp/)
    -> Rust C ABI (native_rust_core/)
librustdesk_core.a
    -> rustdesk_harmony_bridge
    -> upstream RustDesk session/core (rustdesk-master/)
RustDesk server / peer
```

## Structure

| Directory | Description |
|-----------|-------------|
| `native_rust_core/` | Rust bridge layer (`bridge_api.rs`, `bridge_state.rs`, `lib.rs`) |
| `rustdesk-master/` | Upstream RustDesk 1.4.7 source with OHOS patches |
| `patches/` | OHOS-specific crate patches (`machine-uid`) |
| `rdev-fork/` | rdev input-library fork with OHOS support |
| `cpp/` | C++ NAPI bridge layer |
| `scripts/` | Build scripts and code generators |
| `docs/` | Architecture, gap, compliance, and validation documents |

## Build

### Windows

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build_native_bridge.ps1
```

### Linux

```bash
./scripts/build_native_bridge.sh aarch64-unknown-linux-ohos release
./scripts/build_native_bridge.sh x86_64-unknown-linux-ohos release
```

## CI/CD

### Windows online build

- Workflow: `.github/workflows/build-core-windows.yml`
- Runner: `windows-2022`
- Trigger: manual (`workflow_dispatch`)
- Targets: `aarch64-unknown-linux-ohos` + `x86_64-unknown-linux-ohos`
- Assets: `librustdesk_core.a` + `librustdesk_core_x86_64.a`

### Linux online build

- Workflow: `.github/workflows/build-core-linux.yml`
- Auto trigger: `.github/workflows/auto-linux-core-build.yml`
- Runner: `ubuntu-22.04`
- Trigger: automatic after main-branch updates, or manual
- Targets: `aarch64-unknown-linux-ohos` + `x86_64-unknown-linux-ohos`
- Assets: `librustdesk_core.a` + `librustdesk_core_x86_64.a`
- Required repository secret: `OHOS_SDK_LINUX_ZIP_URL`

### Release notes

- Template script: `.github/scripts/write-core-release-notes.sh`
- Auto updater: `.github/workflows/update-core-release-notes.yml`
- `core-001` uses the detailed first-release notes.
- `core-002` and later use the update-release notes template.
- Only the release notes body is updated; tags and release names are not changed.

## Release assets

| File | Architecture | Purpose |
|------|--------------|---------|
| `librustdesk_core.a` | arm64-v8a | HarmonyOS device debugging and integration |
| `librustdesk_core_x86_64.a` | x86_64 | HarmonyOS / OpenHarmony emulator debugging |

Both artifacts must exist and pass size validation before a GitHub Release is created. If any build step fails, the reserved tag remains, but no GitHub Release or release asset is published.

## Usage in HAP projects

1. Download the corresponding static library from GitHub Releases.
2. Copy it into the HAP project's ABI directory, for example `entry/src/main/libs/<ABI>/`.
3. Use it together with this repository's C++ NAPI bridge, ArkTS type declarations, and application-side calling logic.
4. If the bridge layer changed, also sync `cpp/` and `cpp/types/`.

## Capability boundaries

- Outgoing remote sessions use the upstream session path and publish video through `on_rgba -> publish_real_video_frame -> video-frame`.
- The OHOS viewing side uses libvpx software decoding for VP8/VP9 and libyuv for YUV-to-RGBA conversion.
- HarmonyOS controlled-side video transfer has been confirmed on a real device.
- HarmonyOS controlled-side remote input/control is treated as unsupported on the current platform. Do not present input injection as a supported capability or release blocker.
- File transfer, audio, voice calls, recording, screenshots, remote cursor, and complete menu-state behavior still require end-to-end validation.

See `docs/OFFICIAL_CORE_GAP.md` for detailed capability gaps and validation boundaries.

## Compliance notes

- This is not an upstream official project.
- Upstream project names are used only to identify source origin and compatibility targets.
- Static-library redistribution should preserve the corresponding source, build scripts, patches, third-party notices, and license information.
- Third-party notices: `NOTICE`, `docs/THIRD_PARTY_NOTICES.md`.
- Upstream license text: `rustdesk-master/LICENCE`.

## Historical notes

Old test package numbers, historical SHA values, historical package sizes, and historical release IDs are no longer shown as the current primary status. The current release process is the unified `core-XXX` flow described above.
