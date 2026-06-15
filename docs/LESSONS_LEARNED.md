# 经验教训

> 记录容易复发的构建、发布和排查问题。新增经验时优先写清楚：现象、根因、修复、以后如何避免。

## 2026-06-15: Incoming share frame cache is not incoming service readiness

### Symptom

- The Harmony app could start native `OH_AVScreenCapture` and count buffers, but the core had no ABI to receive that payload.
- It was tempting to flip `incomingReady=true` once native buffers appeared.

### Root cause

- Incoming sharing has multiple layers: app screen-capture permission, native buffer acquisition, core frame ingestion, desktop server/video source subscription, and rendezvous/service readiness.
- A frame payload in core memory is necessary progress, but it is not enough for a remote peer to receive RustDesk video frames.

### Fix

- Added an independent `incoming_screen_frame` latest-frame cache in the Harmony bridge and old mirror.
- Added C ABI/NAPI/d.ts functions: `updateIncomingScreenFrame`, `getIncomingScreenFrameMetadata`, `copyIncomingScreenFrame`, and `clearIncomingScreenFrame`.
- Kept `incomingReady=false` while the OHOS desktop server/video source path is still missing.
- Local release build from the real core path passed. Produced `librustdesk_harmony_bridge.a` size `128,711,798` bytes, SHA256 `877AA1B9F27425D07B31193E0CABE6804FDE88AD5F8B622B0F5D52865CC54D5F`.

### Avoidance

- Do not reuse outbound remote-control `latest_video_frame` for incoming share frames; keep directions separate.
- Do not mark incoming service ready until both the desktop server side and the video source subscription can consume the incoming frame payload.
- When adding core frame ingestion, update Rust bridge, C ABI, C++ NAPI, d.ts, app C++ copy, and ArkTS wrapper together.

## 2026-06-15: Direct session functions need status returns and event parity

### Symptom

- RemoteControl UI had native functions for switch sides, screenshot, session recording, and voice call, but some menu actions still used generic option helpers or local Harmony screen capture.
- Several core direct session commands returned `void`, so ArkTS could not distinguish "function exists" from "active session accepted the command".
- Recording, screenshot response, and voice-call state callbacks were empty or incomplete, leaving UI state to local guesses.

### Root cause

- The audit checked API presence before checking end-to-end semantics: Rust bridge return value, C ABI declaration, C++ NAPI wrapper, ArkTS wrapper, and UI caller all have to agree.
- Session recording is a remote-session command, not local app screen capture. Requesting `CUSTOM_SCREEN_CAPTURE` from RemoteControl created a conflict with incoming-share probing.

### Fix

- Converted direct session commands to bool across Rust bridge, C ABI, C++ NAPI, d.ts, and ArkTS wrappers.
- Added `failed=no-active-session` command events when no active session exists.
- Added event callbacks for voice-call started/waiting/incoming/closed, record-status, and screenshot-response.
- Local release build from the real core path passed. Produced `librustdesk_core.a` size `129,028,464` bytes, SHA256 `650E467B3ED67DD368A329FA25BCC024584880FB9B82902C3BE95D2852035E62`.
- 中文发布说明：GitHub Actions run `27516993020` 已发布 `core-79`，用于远控 direct session 命令状态返回和录制/截图/语音事件回流；线上 asset `131,493,470` bytes，SHA256 `8BBB12AA93EE8703ABBED5BA6D411031AD78CE7FA6A71D7C407A0A350A8789F2`。

### Avoidance

- For every core function "接入", verify both directions: UI call path into official `Session`, and official callback/event path back to ArkTS.
- Do not call local screen capture from RemoteControl session recording. Keep local capture reserved for incoming share/probe paths only.
- When a C ABI return type changes, update `bridge_api.rs`, `cpp/rustdesk_bridge_abi.h`, `cpp/rustdesk_bridge_loader.cpp`, core d.ts, app d.ts, and `NativeRustDeskBridge.ts` in one change.

## 2026-06-15: C++ ABI headers must match Rust extern signatures exactly

### Symptom

- The app-side `entry/src/main/cpp` copy already called `rustdesk_bridge_session_send_chat(peer_id, message_type, content, timestamp)`.
- The source-of-truth core project `cpp/rustdesk_bridge_abi.h` still declared `rustdesk_bridge_session_send_chat(const char *content)`, and `cpp/rustdesk_bridge_loader.cpp` still called it with only one argument.
- Rust `native_rust_core/src/bridge_api.rs` exports the function with four arguments, so any future app sync from core would reintroduce chat argument mismatch.

### Root cause

- The earlier fix was applied to the app copy but not fully synchronized back to the core project.
- C/C++ will not protect this path if the local declaration is stale; the loader can compile against the wrong declaration and pass the wrong registers to the Rust ABI.

### Fix

- Updated `cpp/rustdesk_bridge_abi.h` to the four-argument signature: `peer_id`, `message_type`, `content`, `timestamp`.
- Updated `SendChatMessage` and `SessionSendChat` to read `args[2]` as content for four-argument calls, pass all four values to Rust, and retain `args[0]` fallback for legacy one-argument calls.
- Local release build from the real core path passed. Produced `librustdesk_core.a` size `128,882,788` bytes, SHA256 `D0654CC920619957D99E640B7E18969135D224A0F562E26188241B41F47BC45A`.
- 中文发布说明：本轮核心更新用于防止聊天发送 ABI 再次错位，并补齐核心 d.ts 中自定义服务器 `key` 参数，避免下一次同步覆盖 app 副本；GitHub Actions run `27515510727` 已发布 `core-78`，asset `131,470,442` bytes，SHA256 `F68E575D593BBE331E931E582870CB72EAA810BF56B817045162C44FCAF91ACD`。

### Avoidance

- When Rust `#[no_mangle] extern "C"` signatures change, audit all three layers together: `bridge_api.rs`, `cpp/rustdesk_bridge_abi.h`, and `cpp/rustdesk_bridge_loader.cpp`.
- Also compare the 13 core `cpp/` source and the 11 app `entry/src/main/cpp/` copy before publishing, because the app copy can be newer than the core source after emergency fixes. Expected differences should be limited to project-local CMake paths.

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
