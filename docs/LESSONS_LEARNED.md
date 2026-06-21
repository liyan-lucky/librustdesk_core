# 经验教训

> 记录容易复发的构建、发布和排查问题。新增经验时优先写清楚：现象、根因、修复、以后如何避免。

## 2026-06-21：同版本不等于同产物

验收 HAP/Core 时必须联合核对 SHA256、mtime、BuildInfo、双架构 CoreBuildInfo、设备 updateTime 和 hilog，不能只看 `0.33.6`。还要确认两架构来自同一源码时间线：本轮曾发现默认 latest 下载把 2026-06-21 本地 x86_64 覆盖成 2026-06-20 线上资产，及时改为本地固定双架构、扩展 CoreBuildInfo/验包/审计为双架构强制断言，并重跑全部证据。冻结最终哈希后若功能源码、资源、构建配置或 Core 有任何变化，旧证据全部失效并重跑 100 轮最终审计。`99_Temp` 是多项目共享目录，禁止整体清理，所有 APK 永不删除；一次性密码不进入任何持久化介质。华为被控输入是明确搁置边界，不应伪装成已支持。

## 2026-06-21: 构建/测试路径必须统一到 `%VSCODE_ROOT%\99_Temp`

- 散落的 `F:\99_Temp`、仓库内 `.codex_*`、`%TEMP%` 截图和旧 `99_Temp\backups` 会造成 HAP/核心产物新旧混淆，也容易留下含隐私的布局/截图。
- Core 本地构建必须显式设置 `CARGO_TARGET_DIR`、构建缓存和日志目录到 `%VSCODE_ROOT%\99_Temp\librustdesk_core\...`，不要依赖个人临时习惯路径。
- 清理前先确认最新 `.a`、HAP SHA256、BuildInfo/CoreBuildInfo 已写入文档；清理后应能按 `docs/WORKSPACE_PATHS.md` 重新构建。
- 2026-06-21 已按该规则清理：`F:\99_Temp`、旧散落备份、App 仓库 `.codex_*` 和 `%TEMP%` 诊断文件删除；最新 Core/HAP 产物迁移到 `99_Temp`，并创建清理后 App/Core 备份。16:26 二次清理继续删除工作区根 `_tmp_*`、旧 target/HAP/clone/log/cache 和 IDE/工具缓存；保留/删除清单以 `docs/WORKSPACE_PATHS.md` 为准。

## 2026-06-20: option 下发不等于会话功能完成

- 菜单勾选、配置持久化和 `session-option` 事件只证明命令链前半段。
- “显示远程光标”已有 App overlay，但 Core 官方 `Interface` cursor 回调为空，因此没有 cursor data/position 回流，功能仍未实现。
- 所有会话菜单必须同时检查回调/命令、远端实际行为、UI 渲染和断开清理；不得用静态审计或日志行替代设备效果。
- `block-input` 不能通过通用 `get_toggle_option()` 的 option fallback 读取：当前该函数没有对应分支，菜单会错误显示未选中。应以官方 `update_block_input_state` 回调为权威状态，并调查 `unblock-input` 后出现的会话终止事件。
- 文件传输存在 FileManager 调用和 job 事件也不代表端到端完成；必须用真实文件验证目录、双向传输、进度、覆盖、取消、校验和错误恢复。

## 2026-06-19: 双架构核心构建、发布门禁与 IPv4/IPv6 候选地址

### 现象

- GitHub Actions Windows 双架构 run `27848481305` 中 arm64 能继续构建，x86_64 在 libvpx 步骤失败：OHOS SDK `clang` 收到了 nasm/yasm 风格的 `-f elf64` 参数。
- x86_64 libvpx 修好后，Cargo 在 `magnum-opus` 处暴露缺少 `opus/opus_multistream.h`，说明 Opus 没有安装到 `VCPKG_ROOT\installed\<triplet>`。
- release job 使用宽松条件和 `continue-on-error` 下载 artifact，x86_64 失败时仍可能进入发布流程，造成空标签或半成品 release。
- run `27852266805` 首次修复后两个 build job 都成功，但 release job 在检查产物后又执行 `actions/checkout`，checkout 清理了 `./release-assets`，最终创建了无资产的 `core-24` 空 release。
- IPv4-only 手机连接同时拥有 IPv4/IPv6 的客户端时，可能拿到不可用的 IPv6 本地缓存或跨地址族直连候选，导致直连失败后中继兜底不稳定。

### 根因

- libvpx 的 x86 汇编探测会生成 `-f elf64` 等 assembler 参数；当前 OHOS Windows 交叉编译路径把 `AS` 指向 SDK clang，clang 不能消费 nasm/yasm 参数。
- `magnum-opus` 依赖查找路径跟随 `VCPKG_ROOT\installed`，只设置自定义 installed root 或只准备 arm64 依赖会让 x86_64 缺少 Opus 头文件。
- release job 不能用 `always()` 兜底发布；缺少 artifact 存在性和体积检查时，失败矩阵也能污染 latest。
- artifact 检查和 `softprops/action-gh-release` 上传之间不能有会清理工作区的步骤，尤其是默认 `clean: true` 的 `actions/checkout`。
- IPv6 可用性缓存不能只在启动时判断，网络环境切换或当前设备只有 IPv4 时必须重新校验；直连候选必须匹配本地 socket 地址族。

### 修复

- `scripts/build_native_bridge.ps1` 在 `x86_64-unknown-linux-ohos` 构建 libvpx 时禁用 x86 SIMD/汇编路径：`--disable-mmx`、`--disable-sse*`、`--disable-avx*` 等，同时保留 VP8/VP9 encoder/decoder 头文件和 API。
- 构建脚本新增 Opus 1.5.2 静态库准备逻辑，安装到 `VCPKG_ROOT\installed\<triplet>`；x86_64 禁用 Opus intrinsics，避免再引入目标平台汇编问题。
- Windows release job 改为 `if: needs.build.result == 'success'`，下载两个 artifact 不再 `continue-on-error`，发布前强制检查 `librustdesk_core.a` 与 `librustdesk_core_x86_64.a` 同时存在且大小在 `100000000..250000000` bytes。
- release job 移除产物检查后的 checkout，并给 release action 增加 `fail_on_unmatched_files: true`；空 `core-24` release/tag 必须删除后重新触发。
- `test_ipv6()` 在 bind/STUN 失败时清空 `PUBLIC_IPV6_ADDR`；`get_ipv6_socket()` 每次重新 bind 校验；`Client::connect()` 跳过本地地址族与 peer 地址族不一致的直连 TCP/UDP 候选，并在无 relay 参数时使用 rendezvous server + 1 端口作为兜底 relay。

### 验证

- 本地 `x86_64-unknown-linux-ohos` release 构建通过，产物 `128,712,156` bytes，SHA256 `7D0AA289F050AD7D4D06B21516E0B39707570C08A28C700259245EFDA113A1CB`。
- 本地 `aarch64-unknown-linux-ohos` release 构建通过，产物 `130,215,616` bytes，SHA256 `E82E9FE47557EE9771FA5E9C7539EF09670326038F59E8E5748481AE53352B30`。
- x86_64 构建日志确认 libvpx 已成功生成 `libvpx.a`，Opus 已成功生成 `libopus.a`，Cargo build exit code 为 0。
- 线上 run `27853110949` 成功发布 `core-25`；arm64 asset `132,777,178` bytes / SHA256 `EE881BEB9DE44835EE126BACC86D3B373E779334FB58A5D63F4B4D7974077314`，x86_64 asset `130,416,964` bytes / SHA256 `8ACD4AD130EAE9A36D4AE04A93860193CE8773E91E5CCEA5E34E815BFE633ED4`。空 `core-24` release/tag 已删除。

### 以后如何避免

- 不要把 x86_64 OHOS libvpx 当作普通 Linux x86 汇编链路处理；当前 Windows 交叉编译优先禁用 x86 汇编优化，保证可复现产物。
- 新增 native 依赖时同时检查 arm64 和 x86_64 的 `VCPKG_ROOT\installed\<triplet>` 头文件、库文件、pkgconfig/cmake 文件。
- release job 必须以 build matrix 全成功为前置条件，不能用 `always()` 发布 latest；发布前至少检查文件存在、大小范围和 SHA256。
- release job 检查完 artifacts 后不要再 checkout 或清理 workspace；如果确实需要 checkout，必须放在下载 artifact 之前，或在 release 前重新下载/重新检查 artifacts。
- 连接问题遇到 IPv4-only/IPv6-only/双栈混合环境时，先检查本地 socket 地址族和 peer 地址族是否匹配，不要复用过期 IPv6 缓存。

## 2026-06-16: OHOS 替代文件移入 `harmony_bridge/` 子目录

### 现象

- `rustdesk-master/src/` 下有 9 个 `*_ohos.rs` 文件和官方同名模块混在一起，难以区分哪些是官方代码、哪些是 OHOS 专属。
- `libs/scrap/src/common/` 下有 3 个 `*_ohos.rs` 同样与官方文件混杂。

### 根因

- 早期开发时 OHOS 替代文件直接放在同级目录，通过 `#[path = "xxx_ohos.rs"]` 引用。
- 随着文件增多，与官方代码的边界越来越模糊。

### 修复

- 将 `src/` 下 9 个 `*_ohos.rs` 移入 `src/harmony_bridge/` 目录。
- 将 `libs/scrap/src/common/` 下 3 个 `*_ohos.rs` 移入 `libs/scrap/src/common/harmony_bridge/` 目录。
- 更新所有 `#[path = ...]` 引用路径。
- 新增 `docs/OHOS_CODE_MAP.md` 记录完整的 OHOS 代码分布。

### 以后如何避免

- 新增 OHOS 替代文件时，统一放入 `harmony_bridge/` 子目录，不要放在同级。
- 散布在官方文件中的 `cfg(target_env = "ohos")` 条件编译块无法移入独立目录，更新源码时需逐文件合并。
- 更新官方源码前，先备份 `harmony_bridge/` 目录和 `OHOS_CODE_MAP.md`，更新后按文档恢复。

## 2026-06-16: Linux 在线构建需要独立的 OHOS SDK（Linux 版）

### 现象

- Windows 在线构建使用 Windows 版 OHOS SDK（含 `clang.exe`、`llvm-ar.exe` 等），无法在 Linux runner 上使用。
- Linux 构建需要 Linux 版 OHOS SDK（含 `clang`、`llvm-ar` 等原生 ELF 二进制）。

### 根因

- OHOS SDK 是平台相关的：Windows SDK 的二进制是 PE 格式，Linux SDK 的二进制是 ELF 格式。
- 当前仓库密钥 `OHOS_SDK_ZIP_URL` 指向 Windows 版 SDK，Linux 构建需要单独的密钥 `OHOS_SDK_LINUX_ZIP_URL`。

### 修复

- 新增 `.github/workflows/build-core-linux.yml`，使用 `ubuntu-22.04` runner。
- Linux 构建仅手动触发（`workflow_dispatch`），不自动触发。
- 需要在仓库密钥中设置 `OHOS_SDK_LINUX_ZIP_URL`，指向 Linux 版 OHOS Native SDK 压缩包。
- Linux 构建发布标签格式为 `core-linux-*`，与 Windows 的 `core-*` 区分。
- Linux 构建直接在 bash 中编译 libsodium/libvpx/libyuv/opus，不需要 MSYS2。

### 以后如何避免

- 不要假设 OHOS SDK 跨平台通用；Windows 和 Linux 构建需要各自平台的 SDK。
- Linux 构建不需要 MSYS2，直接用系统 bash/perl/make 即可，构建速度更快。
- 如果 Linux 构建失败，先检查 `OHOS_SDK_LINUX_ZIP_URL` 密钥是否已设置且 URL 有效。

## 2026-06-16: GitHub Release 发布说明应默认中文

### 现象

- 部分 Release（如 core-80）的发布说明为英文，与项目文档的中文默认不一致。

### 根因

- Windows workflow 的 `softprops/action-gh-release` 未设置 `body` 字段，导致发布说明为空或英文。
- 早期版本未统一发布说明语言。

### 修复

- 所有 workflow 的 Release 创建步骤现在包含中文默认发布说明模板。
- 已将所有现有 Release 的标题和说明更新为中文。

### 以后如何避免

- 新建 workflow 或修改 Release 步骤时，确保 `body` 字段包含中文说明。
- Release 标题格式统一为 `librustdesk_core 构建 {编号}`（Windows）或 `librustdesk_core Linux 构建 {编号}`（Linux）。

## 2026-06-15: OHOS share capture needs `captureRequired`, not fake `incomingReady`

### Symptom

- After core-80, the app could push native screen-capture payload into the core incoming frame cache.
- If the app waited for `incomingReady=true` before starting native capture, the core would wait for the first frame while the app waited for readiness.
- If the core flipped `incomingReady=true` once a frame existed, the UI and remote peers would see a fake running service before the desktop server/video source was really ready.

### Root cause

- The incoming share state needed a middle signal between "service requested" and "service ready".
- `incomingReady` is externally visible service readiness. It cannot also mean "please start capture now".
- OHOS `scrap::common::ohos::Capturer` was still a stub, so the incoming frame cache was not a real frame source for the RustDesk capture path.

### Fix

- Added `captureRequired` to the Harmony core snapshot. `main_start_service(true)` returns `captureRequired=true`, `incomingReady=false`, and waits for the app to provide a live frame.
- Implemented an OHOS `scrap` incoming frame source: `Display::primary/all` return usable display metadata, and `Capturer::frame()` returns the latest incoming cache payload as `Frame::PixelBuffer`.
- Kept `incomingReady=false` until the desktop server/video source is actually ready to serve remote peers.
- Local release build from the real core path passed. Produced `librustdesk_harmony_bridge.a` size `128,894,588` bytes, SHA256 `2DC3B655664B756E255684D28FBA0CB3A9DEC14E6080EA4682FA26486ADF9B6D`.
- 中文发布说明：GitHub Actions run `27563925971` 已发布 `core-81`，用于 `captureRequired` 中间态和 OHOS scrap incoming frame source；线上 asset `131,631,706` bytes，SHA256 `64463fa57005cd5ccd99bafa9a40f18a9d605f8e90f5e199f92b38abfcdb4829`。

### Avoidance

- Use three separate meanings: `captureRequired` starts app capture, `incomingFramePayloadReady` proves a frame exists in core memory, and `incomingReady` means the remote-facing service is ready.
- Do not start Harmony screen recording from screenshot permission APIs or `AVScreenCaptureRecorder` probes.
- When touching incoming share, update both active bridge and `src/harmony_bridge/harmony_bridge/core.rs` mirror so future copy/sync work cannot regress the state contract.

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

- Historical local cold validation in `L:\Visual_Studio_Code\99_Temp\rustdesk_harmonyos_extra_cxx_validate` rebuilt `libvpx.a` (`3,302,304` bytes) and `libyuv.a` (`683,472` bytes), then completed the full Cargo build. Current work must use `%VSCODE_ROOT%\99_Temp` / `F:\Visual_Studio_Code\99_Temp` instead of recreating the old L: path.
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

## 2026-06-20: Isolate stale Harmony sessions by generation

### What happened

- VM evidence showed a new password connection reaching connected, then an older session thread emitted `Reset by the peer` and changed the shared state to error.
- App-side event deduplication was insufficient because the stale Rust callback mutated core state before ArkTS saw the event.
- A password accepted without explicit App persistence could remain in upstream `PeerConfig`, making a later request appear to bypass password input.

### Fix and rule

- Every Harmony session handler carries a monotonically increasing generation. Starting or closing a session invalidates all previous handlers.
- Guard state-changing callbacks, queued events, quality updates and RGBA publication with `is_current()`; do not only guard `on_connected()`.
- Clear upstream peer password before a new Harmony request. The App decides whether a password is remembered and passes an explicit password when required.
- Treat `peer-info`, `connection-type` and fingerprint as metadata. Only the official connected callback may publish authenticated session state.

## 2026-06-20: Parallel target builds need unique log paths

- Timestamp-to-second log names collide when arm64 and x86_64 scripts start together, causing a misleading stream-read failure before Cargo runs.
- Include target triple and milliseconds in every per-run log file. Parallel local validation must be part of any build-script change that claims dual-architecture support.
