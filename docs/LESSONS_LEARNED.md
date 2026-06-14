# 经验教训

> 记录容易复发的构建、发布和排查问题。新增经验时优先写清楚：现象、根因、修复、以后如何避免。

## 2026-06-14: Keep inactive Harmony source mirrors from regressing working paths

### Symptom

- The active `rustdesk-master/src/harmony_bridge/core.rs` already sent text clipboard data by building a RustDesk `Clipboard` protobuf and calling `session.send(Data::Message(...))`.
- The older mirror at `rustdesk-master/src/harmony_bridge/harmony_bridge/core.rs` still returned `false` from `send_clipboard_data()`.

### Root cause

- Previous audits focused on the active compile path, while the repository still keeps an older Harmony bridge copy that can be used for comparison or future synchronization.
- A stale stub in that mirror can reintroduce a bug if code is copied back from the wrong file.

### Fix

- Updated the old mirror `send_clipboard_data()` to match the active implementation: require an active session, build `Clipboard { format: Text, content, compress: false }`, wrap it in `Message`, and send it through the official session.
- Local release build from the real core path passed after the change; the produced active artifact hash stayed unchanged because this mirror is not currently part of the compiled path.
- Release validation for this mirror sync: commit `1b987914a2c27ace376e5af45a9c6790d84d40b4`, GitHub Actions run `27486100946`, release `core-74`, asset size `131,471,786` bytes, SHA256 `3755D448FBB1A583E7B5F7C3C6ADEC29D8AF0FBB7E5DD192251CD18A68C45D7C`. The app full HAP build/package verification/install passed after downloading core-74 as version `0.19.0`; runtime launch was blocked by device lock screen, not by the core artifact.

### Avoidance

- When a feature is fixed in `src/harmony_bridge/core.rs`, grep the old `src/harmony_bridge/harmony_bridge/core.rs` mirror for the same function before publishing.
- If the mirror is intentionally inactive, still keep user-visible behavior stubs aligned for clipboard, terminal, file transfer, and session command paths.

## 2026-06-14: File transfer needs callback-event parity, not just API parity

### Symptom

- The Harmony app had `FileTransferService.ets`, NAPI wrappers, and C ABI functions for reading remote directories, creating directories, deleting paths, and starting transfers.
- The app listened for `folder-files`, `file-transfer-start`, `job-progress`, `job-done`, `job-error`, `create-remote-dir`, and `delete-remote-path`, but the core did not emit most of those events.
- `session_send_files()` called official `send_files()` with a generated job id, then emitted a generic event without that id, so the app could not associate progress with the task.

### Root cause

- Interface-name parity was checked, but `InvokeUiSession` file-transfer callbacks in the Harmony handler were still empty.
- Create/delete/start paths emitted generic `file-transfer` events while the ArkTS side had already split them into action-specific events.
- File-transfer is a bidirectional workflow: start calls must be paired with job callbacks and directory listing callbacks.

### Fix

- `HarmonyHandler` now emits `job-error`, `job-done`, `job-progress`, `clear-all-jobs`, `update-transfer-list`, `load-last-job`, `folder-files`, `update-folder-files`, `confirm-delete-files`, and `override-file-confirm`.
- `update_folder_files()` uses `crate::common::make_fd_to_json(...)` for full directory entries and count-only JSON for `only_count`.
- `session_send_files()` stores one `job_id`, passes it to official `send_files()`, and emits `file-transfer-start` with the same id.
- `session_create_dir()` and `delete_remote_path()` emit `create-remote-dir` and `delete-remote-path` respectively.

### Avoidance

- For every app-visible workflow, audit both call direction and callback/event direction. A wrapper existing in `NativeRustDeskBridge.ts` is not proof the feature is complete.
- Event names must match the app listener contract exactly; do not hide distinct operations behind a generic event if ArkTS routes by event kind.
- When the app has both a direct session function and a generic option helper, audit the UI's actual path. The RemoteControl "Switch Sides" menu uses `applySessionOption('switch-sides', 'Y')`, so the core option route must call `Session::switch_sides()` even though `session_switch_sides()` also exists.
- Local validation for this fix before push: `scripts\build_native_bridge.ps1 -Profile release` from the real core path passed; produced `librustdesk_core.a` size `128,994,138` bytes, SHA256 `24F7729894862CD9ACBC44266C03563CDD8C9E2CC1AC81D0827A22E89C7A181F`.
- Release validation for this fix: commit `275b231e11aefd4a2e51050fc74fbdeba9c566bd`, GitHub Actions run `27485061967`, release `core-73`, asset size `131,471,532` bytes, SHA256 `E444D739EC958CD1485519FE0A712BFC1F074B60EEA65D71552E7E95A909A7B1`. The app full HAP build/package verification/install passed after downloading core-73; runtime launch was blocked by device lock screen, not by the core artifact.

## 2026-06-14: Build from the real core path, not the app junction

### Symptom

- Running `scripts\build_native_bridge.ps1 -Profile release` from `11_Rustdesk_harmonyos\13_librustdesk_core` failed before Cargo with a missing vcpkg installed root under `11_Rustdesk_harmonyos\99_Temp\rustdesk_harmonyos_build\vcpkg\installed`.
- The same command succeeded from the real project path `%VSCODE_ROOT%\13_librustdesk_core`.

### Root cause

- The app project contains `13_librustdesk_core` as an NTFS junction for source browsing.
- The core build scripts derive workspace/build roots from the current project path. Starting from the junction makes the script infer the wrong parent and use an app-local `99_Temp`.

### Fix

- Always run core builds, commits, pushes, and release checks from `%VSCODE_ROOT%\13_librustdesk_core`.
- Treat `11_Rustdesk_harmonyos\13_librustdesk_core` as a convenience link only.

### Avoidance

- If vcpkg, OHOS SDK, or build-cache paths unexpectedly include `11_Rustdesk_harmonyos\99_Temp`, first check the current working directory.
- Do not document or automate core builds using the app-junction path.

## 2026-06-14: Terminal bridge and media event payloads

### Symptom

- The Harmony app had `Terminal.ets`, `TerminalService.ets`, NAPI declarations, and C ABI wrappers, but terminal open/input/resize/close still failed.
- Remote terminal output would be unsafe to place in the session-event JSON `detail` field as raw text because it can contain ANSI/control bytes.
- `pull_audio_frames_json()` returned `{}` for an empty queue, while the app audio poller expects an array.
- App chat used four arguments, but the core project C++ bridge still read `args[0]`, which can turn peer id into the message body.

### Root cause

- Interface-name parity was checked, but the Harmony Rust bridge implementation still had terminal functions returning `false`.
- The event bus is JSON text, not a binary-safe terminal transport.
- Empty media queues must match the consumer contract; `{}` and `[]` are not interchangeable.
- The App project had already fixed C++ chat argument reading, but the source-of-truth core project had not been synchronized.

### Fix

- `rustdesk-master/src/harmony_bridge/core.rs` now forwards terminal open/input/resize/close to official `Session`.
- `HarmonyHandler.handle_terminal_response()` emits `terminal-response`, `terminal-output`, and `terminal-closed`; terminal data is decompressed if needed and base64 encoded as `dataBase64`.
- `pull_audio_frames_json()` returns `[]` when no frames are available.
- `cpp/rustdesk_bridge_loader.cpp` reads chat content from `args[2]` for four-argument calls, with `args[0]` fallback for old one-argument calls.

### Avoidance

- For any "App has UI but feature does not work" issue, verify ArkTS -> NAPI -> C ABI -> Rust bridge -> official Session -> event return path.
- Do not put raw binary/control-byte payloads into `queue_event()` detail; encode them first.
- Keep 13 core C++ bridge and 11 App C++ bridge in sync before publishing a new core.
- Release validation for this fix: commit `38c837cee0bb28aee795c0fc3895044f1440f96a`, GitHub Actions run `27483922931`, release `core-71`, asset SHA256 `C750A785297AA22A2518B158BF334A1B1415C4E0739E01D0856C8BB5D450E15C`.

## 2026-06-13: libvpx RTC C++ targets and OHOS libc++ include paths

### Symptom

- GitHub Actions runs `27451105187` and `27452113153` failed before Cargo while building `libvpx 1.15.2`.
- The failing files were `vp9/ratectrl_rtc.cc` and `vp8/vp8_ratectrl_rtc.cc`; both failed on `fatal error: 'cstdint' file not found`.
- Artifact `7605933947` contained only `build_debug_20260613_011822.log`, confirming this was an early libvpx cold-build failure, not a Rust/Cargo failure.
- Online run `27458902852` selected MSYS2 libc++ headers after resolving the correct setup-msys2 root, but those headers were too new for the OHOS SDK clang and failed inside libc++ type-trait builtins.

### Root cause

- The OHOS sysroot does not make libc++ headers visible to clang++ by itself.
- Any `libvpx` C++ source that is actually built must receive a compatible SDK libc++ include directory in the generated C++ build flags, not only through loosely assumed environment state.
- When Windows `clang++.exe` is invoked from MSYS, a glued argument such as `-isystem/msys/path` is not path-converted. Use `-isystem /msys/path` as two arguments, or a native/forward-slash Windows path.
- The online SDK zip can be a minimal package that has `native/llvm` and `native/sysroot` but no SDK libc++ headers. MSYS2 libc++ is not a safe fallback because its headers can require a newer clang than the OHOS SDK provides.
- The files `vp9/ratectrl_rtc.cc` and `vp8/vp8_ratectrl_rtc.cc` are pulled in by the unused `libvpxrc.a` target. They are not required for the `libvpx.a` C API that `scrap` links.
- `msys2/setup-msys2` may install packages into the action-managed MSYS2 root while a separate preinstalled `C:\msys64` also exists. Seeing `C:\msys64\usr\bin\bash.exe` does not prove it is the MSYS2 instance that just installed `mingw-w64-clang-x86_64-libc++`.
- Do not disable libvpx VP8/VP9 encoders as a shortcut unless `scrap` is changed too. `scrap/src/bindings/vpx_ffi.h` includes `vpx/vp8cx.h` and `vpx/vpx_encoder.h`, and `scrap/src/common/vpxcodec.rs` references encoder APIs.

### Fix

- Resolve the SDK libc++ include directory by checking for `cstdint`, but treat it as optional for `libvpx`.
- Build `libvpx` with `make libvpx.a` and manually install `libvpx.a`, the public `vpx/*.h` headers, and `lib/pkgconfig/vpx.pc`.
- Do not run `make && make install` for `libvpx` on CI, because that path builds unused `libvpxrc.a` and requires C++ RTC sources.
- Remove the MSYS2 libc++ package/fallback from the workflow; keep MSYS2 only for bash/perl/cygpath/build tooling.
- Keep only one libc++ include root. Do not add both `include/c++/v1` and `include/libcxx-ohos/include/c++/v1`.

### Validation

- Local cold validation in `L:\Visual_Studio_Code\99_Temp\rustdesk_harmonyos_extra_cxx_validate` rebuilt `libvpx.a` (`3,302,304` bytes) and `libyuv.a` (`683,472` bytes), then completed the full Cargo build.
- Produced local core: `129,593,638` bytes, SHA256 `2322E55089629C7CB9FFD426481220BDD43AB3C3DA46F37D85AD0A85DD5ADDFB`.
- Online run `27458205351` proved the MSYS2 package was installed, but the script checked the wrong root (`C:\msys64`). The follow-up fix probes `msys2.cmd` first and only then falls back to PATH or `C:\msys64`.
- Local validation after the MSYS2-root fix used `C:\rustdesk_harmony_decoder_validate`: an attempted decoder-only libvpx build produced `libvpx.a` but failed Cargo because `vp8cx.h` was not installed; restoring full encoder support produced `libvpx.a` (`3,302,224` bytes), full Cargo exit code `0`, and local core `129,592,014` bytes, SHA256 `32F3B3AC37EC82C94F2B4B3BA041459D2AF8ADA6AB1F3A57B39056F460F61B5F`.
- Local validation after switching to `make libvpx.a` used `C:\rustdesk_harmony_decoder_validate`: `libvpx-build.log` contained no `[CXX]`, no `ratectrl_rtc`, and no `libvpxrc`; the full Cargo build exited `0`. Produced `libvpx.a` (`3,302,224` bytes), `libyuv.a` (`683,472` bytes), and core `129,592,014` bytes, SHA256 `32F3B3AC37EC82C94F2B4B3BA041459D2AF8ADA6AB1F3A57B39056F460F61B5F`.

## 2026-06-12：GitHub Actions 误发布 dev profile 静态库

### 现象

- 手工上传的 `v1.4.7-ohos` release asset：
  - URL: `https://github.com/liyan-lucky/librustdesk_core/releases/download/v1.4.7-ohos/librustdesk_core.a`
  - Size: `138,394,514` bytes (`131.98 MiB`)
- push 后 GitHub Actions 生成的 `core-62` release asset：
  - URL: `https://github.com/liyan-lucky/librustdesk_core/releases/download/core-62/librustdesk_core.a`
  - Size: `595,083,124` bytes (`567.52 MiB`)
- 两者源码目标相同，但体积相差约 4.3 倍。

### 根因

`.github/workflows/build-core.yml` 的 `Build librustdesk_core.a` 步骤曾调用：

```powershell
.\scripts\build_native_bridge.ps1 -TargetTriple "$env:TARGET_TRIPLE" -Profile dev
```

PowerShell 构建脚本会把 `dev` 映射到 Cargo 的 `debug` 输出目录，因此 CI 发布的是 dev/debug staticlib。该 profile 会保留大量 debug 信息，`staticlib` 体积会显著大于 release 产物。

同时，workflow 还曾设置：

```text
CARGO_PROFILE_RELEASE_LTO=false
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
CARGO_PROFILE_RELEASE_STRIP=false
```

这些变量会覆盖 `rustdesk-master/Cargo.toml` 中的 `[profile.release]` 配置，容易让 CI release 与本地 release 不一致。

### 修复

- GitHub Actions 改为调用 `-Profile release`。
- 删除 workflow 中覆盖 release profile 的 `CARGO_PROFILE_RELEASE_*` 变量，让 `Cargo.toml` 作为 release 配置权威来源。
- 发布前新增体积闸门：
  - 最小：`100,000,000` bytes
  - 最大：`250,000,000` bytes
  - 当前正常基准约 `132 MiB`。

### 以后如何避免

- 发布到 GitHub Release 的 `librustdesk_core.a` 必须来自 Cargo `release` profile。
- 看到 `.a` 接近 `568 MiB` 时，第一反应应检查 workflow profile，而不是先怀疑源码膨胀。
- 不要为了临时排查把 `-Profile dev` 留在发布 workflow；如需 debug 产物，应上传到单独 artifact，不要进入 release asset。
- 不要随意在 workflow 中设置 `CARGO_PROFILE_RELEASE_*` 覆盖项；必须覆盖时同步更新 `CORE.md` 和本文件。
- 每次替换 HAP 项目的 native core 前，至少检查 size 和 SHA256。
