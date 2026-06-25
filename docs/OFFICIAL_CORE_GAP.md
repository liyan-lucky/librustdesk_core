# RustDesk 官方核心对齐审计

> 2026-06-26 对齐基线。本文只记录当前 HarmonyOS 核心相对 RustDesk 官方完整核心/Flutter FFI/桌面被控端能力的差距、验收边界和补齐顺序。用户已实测：HarmonyOS 被控端画面传输已实现；HarmonyOS 被控端远程输入/操控当前按平台不支持处理；其他非画面能力尚未逐项实测。

## 状态定义

| 状态 | 含义 |
|------|------|
| 已接通 | 已走 RustDesk official session/core 路径，且至少有一次真实端到端验证或构建验证。 |
| 已实现待复验 | 代码路径已补齐，但缺少当前基线下的端到端复验。 |
| 未测 | 不能根据函数名、符号或构建通过推定可用，必须补真实行为测试。 |
| 平台不支持 | 当前 HarmonyOS 普通应用能力不足，不能作为发布阻塞项，也不能在 UI 中宣称支持。 |
| 待补齐 | 需要补 Rust/C++/ArkTS 或平台适配，当前不能按官方能力对外承诺。 |

## 当前权威结论

| 模块 | 当前结论 | 验收边界 |
|------|----------|----------|
| 出站远控连接 | 已接通 | `connect_to_peer()` 必须走 official `Session`；不能回退旧 stub。 |
| 出站远控画面 | 已接通 | `on_rgba -> publish_real_video_frame -> video-frame`，App 能持续拉到真实 RGBA 帧。 |
| 出站远控输入 | 已接通待复验 | ArkTS mouse mask 必须按官方编码；键鼠命令必须转发到 active session 并返回真实状态。 |
| HarmonyOS 被控端画面 | 已实测可用 | HarmonyOS 真机作为被控端，Windows 端可看到真实持续刷新画面。 |
| HarmonyOS 被控端输入/操控 | 平台不支持 | 不作为发布阻塞项；UI/状态不得宣称可控；符号仅作兼容、诊断和明确失败边界。 |
| 文件传输 | 已实现待复验 | 事件已接入不等于可用；必须补双向上传/下载/目录/覆盖/删除/大文件测试。 |
| 终端 | 已接通待复验 | `session_open_terminal/session_send_terminal_input/session_resize_terminal/session_close_terminal` 必须走 official `Session`；输出需以 `dataBase64` 回流。 |
| 聊天 | 已接通待复验 | C ABI/C++/d.ts 四参签名必须保持一致，旧一参调用只能作为兼容 fallback。 |
| 剪贴板 | 已接通待复验 | 不能保留 `false` stub；需验证文本、空内容、中文、跨端方向。 |
| 远程光标 | 待补齐/待复验 | `set_cursor_data/set_cursor_id/set_cursor_position/set_display/main_set_cursor_position` 必须有事件回流和 UI 展示验证。 |
| 菜单状态 | 待复验 | `switch-sides/block-input/privacy-mode/record/voice/screenshot/display` 不能只发命令，必须能回读状态和失败原因。 |
| 音频 | 未测 | `pull_audio_frames_json()` 空队列必须返回 `[]`；采集、播放、格式、延迟均需真测。 |
| 语音通话 | 未测 | started/waiting/incoming/closed 事件已补不等于可用；需双端呼叫流程测试。 |
| 录制 | 未测 | record 命令、状态回流、文件保存路径、权限和失败原因均需验证。 |
| 截图 | 未测 | screenshot response、文件保存、权限、错误回流需验证。 |
| 编解码 | 已接通待复验 | VP8/VP9 + libvpx/libyuv 路线可用时才声明；不能虚标 VP9。 |
| LAN/relay/IPv4/IPv6 | 已实现待复验 | 混合地址族、无 relay 兜底、relay 连接和重连需要同一基线重测。 |
| Linux 在线构建 | 已接入待持续验证 | 使用 `OHOS_SDK_LINUX_ZIP_URL`，产物路径必须从 `CARGO_TARGET_DIR` 查找，标签为 `core-linux-*`。 |

## 必须保持的核心边界

1. `harmony_bridge/core.rs` 是 HarmonyOS 专用入口，不是官方 `flutter_ffi.rs` 的一比一复制。每次上游升级都必须做 API/回调对照。
2. `incomingReady` 不能表示“完整被控能力”。应至少拆分为：
   - `screenReady`：画面链路可服务。
   - `inputReady`：输入/操控链路可用；当前 HarmonyOS 为 `false/unsupported`。
   - `captureRequired`：核心等待 App 启动录屏并推首帧。
   - `serviceReady`：rendezvous/server/socket 层可服务。
3. HarmonyOS 输入注入当前按平台不支持处理，不再作为发布阻塞项。
4. 文件传输、音频、语音、录制、截图、远程光标、菜单状态必须以真实端到端行为作为完成标准，不能以 NAPI 函数存在、符号导出或构建通过作为完成标准。
5. 所有命令型 API 必须返回真实执行状态；无 active session 时返回失败并发可诊断事件，不能静默 `true`。
6. 旧 mirror 路径 `rustdesk-master/src/harmony_bridge/harmony_bridge/core.rs` 不能落后于 active bridge 中已接通的功能，避免未来同步/生成时退回 stub。

## 极限对齐审计清单

### A. Official Session / FFI 对照

- [ ] 列出 RustDesk 1.4.7 official `Session`、`InvokeUiSession`、`flutter_ffi` 暴露能力。
- [ ] 对照 `harmony_bridge/core.rs`、`native_rust_core/src/bridge_api.rs`、`cpp/rustdesk_bridge_abi.h`、`cpp/rustdesk_bridge_loader.cpp`、`cpp/types/librustdesk_bridge/index.d.ts`。
- [ ] 每个函数标记：已接通、stub、平台不支持、未暴露、已暴露未测。
- [ ] 所有 ArkTS 调用路径确认是否走 direct API 还是 generic option helper。

### B. 出站远控验收

- [ ] Windows/Linux/Android 远端各连一次。
- [ ] 自建服务器 key、relay、直连分别验证。
- [ ] 鼠标移动、左/右键、滚轮、键盘、Ctrl+Alt+Del 验证。
- [ ] 远程光标显示/隐藏验证。
- [ ] 多显示器切换验证。
- [ ] 断网重连、Wrong Password 后重试验证。

### C. HarmonyOS 被控端验收

- [x] 画面传输：HarmonyOS 真机作为被控端，Windows 端能看到持续刷新画面。
- [ ] 画面持续 5 分钟以上无卡死、无 panic、无 fatal。
- [ ] 锁屏/息屏/后台/前台切换后的行为记录。
- [ ] relay 模式被控画面验证。
- [ ] 重连后首帧缓存清理验证。
- [ ] 输入/操控：当前平台不支持，UI 必须明确显示 unsupported，不给用户可控预期。

### D. 文件传输验收

- [ ] Windows -> HarmonyOS 下载文件。
- [ ] HarmonyOS -> Windows 上传文件。
- [ ] 文件夹列表、进入目录、返回上级。
- [ ] 新建远程目录。
- [ ] 删除远程路径。
- [ ] 覆盖确认。
- [ ] 中文文件名。
- [ ] 大文件进度。
- [ ] 权限不足和断线失败回流。

### E. 剪贴板/聊天/终端

- [ ] 聊天四参调用和旧一参 fallback 都不破坏内容。
- [ ] 剪贴板双向文本、中文、空字符串。
- [ ] 终端打开、输入、resize、关闭。
- [ ] 终端二进制/控制字符必须通过 `dataBase64`，不能破坏 JSON。

### F. 音频/语音/录制/截图

- [ ] `pull_audio_frames_json()` 空队列返回 `[]`。
- [ ] 远端音频播放。
- [ ] 语音呼叫完整状态流。
- [ ] 录制开始/停止/状态/文件保存。
- [ ] 截图请求/响应/保存/错误回流。

### G. 构建/发布

- [ ] Windows 双架构 `core-*` 仍为主发布路径。
- [ ] Linux arm64 `core-linux-*` 可手动构建并验证产物大小。
- [ ] 每次发布前检查 `.a` 体积在 `100,000,000` 到 `250,000,000` bytes。
- [ ] 每次 HAP 打包后核对 CoreBuildInfo 的大小、mtime、hash。
- [ ] 不用 dev profile 产物；异常 568 MiB 级别产物一律拒绝。

## 当前优先级

1. 更新 App UI/状态模型，明确被控端“画面可用、输入不支持、其他未测”。
2. 补 `screenReady/inputReady/serviceReady/captureRequired` 的状态拆分，避免 `incomingReady` 含义过载。
3. 做 HarmonyOS 被控端 5 分钟稳定性、relay、重连、锁屏/后台测试。
4. 做文件传输双向完整测试。
5. 做远程光标、菜单状态、剪贴板、终端复验。
6. 最后再推进音频、语音、录制、截图。

## 禁止回退项

- 不能把已经走 official `Session` 的函数退回 stub。
- 不能把 HarmonyOS 输入注入伪装成可用。
- 不能仅凭 native buffer 有帧就宣称完整被控端 ready。
- 不能仅凭符号导出就宣称文件传输/语音/截图/录制完成。
- 不能让旧 mirror core 覆盖 active bridge 的已修复逻辑。
