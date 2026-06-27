# RustDesk 官方核心对齐审计

> 2026-06-26 对齐基线。本文记录当前 HarmonyOS 核心相对 RustDesk 官方完整核心 / Flutter FFI / 桌面被控端能力的实现差距、未实现项、平台限制和验收边界。用户已实测：HarmonyOS 被控端画面传输已实现；HarmonyOS 被控端远程输入/操控当前按平台不支持处理；其他非画面能力尚未逐项实测。

## 状态定义

| 状态 | 含义 |
|------|------|
| 已实现 | 当前 HarmonyOS 核心已经走 RustDesk official session/core 路径，且至少有一次构建或端到端验证支撑。 |
| 已实现待复验 | 代码路径已补齐，但缺少当前基线下的真实端到端复验。 |
| 部分实现 | 只完成桥接、事件、状态或单方向能力，尚不能等同官方完整能力。 |
| 未实现 | 当前缺少关键 Rust/C++/ArkTS/平台链路，不能按官方能力对外承诺。 |
| 未测 | 不能根据函数名、符号或构建通过推定可用，必须补真实行为测试。 |
| 平台不支持 | 当前 HarmonyOS 普通应用能力不足，不能作为发布阻塞项，也不能在 UI 中宣称支持。 |
| 禁止回退 | 已经接通 official 路径的能力，后续同步、生成或升级时不得退回 stub。 |

## 总体结论

当前 HarmonyOS 核心不是空壳，已经接入 RustDesk official session/core，并形成 `ArkTS UI -> NAPI -> C++ bridge -> Rust C ABI -> librustdesk_core.a -> RustDesk official session/core` 的链路。核心差距不在“能不能构建官方 core”，而在以下几个方面：

1. HarmonyOS 使用 `harmony_bridge/core.rs` 作为专用入口，不是官方 `flutter_ffi.rs` 的一比一复制，因此每次上游升级都需要重新对照官方 FFI、Session API 和 UI handler 回调。
2. HarmonyOS 被控端画面传输已实测可用，但远程输入/操控受当前平台能力限制，按不支持处理。
3. 文件传输、终端、聊天、剪贴板、菜单命令等已经有桥接或事件路径，但仍需要按官方行为做端到端复验。
4. 音频、语音、录制、截图、远程光标、完整菜单状态属于高风险区，不能仅凭符号导出判断完成。
5. `incomingReady` 不能再表示“完整被控能力”，必须拆成画面、输入、服务、采集等独立状态。

## 官方能力 vs HarmonyOS 当前实现矩阵

| 官方能力模块 | 官方核心/桌面端预期 | HarmonyOS 当前实现 | 当前状态 | 未实现/差距 | 验收标准 |
|--------------|--------------------|--------------------|----------|-------------|----------|
| 基础连接 / Session 启动 | `session_start`/official `Session` 建立连接，支持密码、key、自建服务器、直连/relay。 | `connect_to_peer()` 已要求走 official `Session`，不能回退 stub。 | 已实现待复验 | 需要同一当前基线重新验证密码错误后重试、key、自建服务器、relay、IPv4/IPv6。 | Windows/Linux/Android 远端分别连接成功；Wrong Password 后可继续输入密码进入会话。 |
| 出站远控画面 | 控制端能接收远端图像并持续刷新。 | 通过 `on_rgba -> publish_real_video_frame -> video-frame` 输出，OHOS 侧使用 libvpx/libyuv 软解码。 | 已实现 | VP9 只能在真实解码可用时声明，不能虚标。 | 连接后持续获得真实 RGBA 帧，切换质量/编码不崩溃。 |
| 出站远控鼠标键盘 | 鼠标、键盘、滚轮、组合键走 official session。 | 鼠标 mask 要按官方编码；`send_mouse_input()`、`send_ctrl_alt_del()` 要走 active session。 | 已实现待复验 | 需要验证左/右键、滚轮、拖拽、键盘、Ctrl+Alt+Del、中文输入边界。 | 每个输入动作远端真实响应，失败时返回 false 或事件，不静默成功。 |
| HarmonyOS 被控端画面 | 被控端启动服务后，对端能看到本机屏幕。 | 用户已实测 HarmonyOS 真机作为被控端，Windows 端可看到真实持续刷新画面。 | 已实现 | 还需补 5 分钟稳定性、relay、断线重连、锁屏/后台行为。 | Windows 官方客户端可看到 HarmonyOS 屏幕持续刷新，无 fatal/panic。 |
| HarmonyOS 被控端输入/操控 | 对端能远程控制被控端鼠标/触摸/键盘。 | 当前按 HarmonyOS 平台不支持处理；输入注入符号仅作兼容和诊断边界。 | 平台不支持 | 不应继续追求普通应用输入注入闭环，不作为发布阻塞项。 | UI 明确显示 input unsupported；远端不能误以为可控。 |
| 入站服务状态 | 服务 ready 状态准确反映可连接、可看画面、可输入。 | 现有 `incomingReady/captureRequired` 已有历史演进，但语义需要拆分。 | 部分实现 | `incomingReady` 含义过载，容易把“画面可用”误报成“完整被控能力”。 | 拆成 `serviceReady/screenReady/inputReady/captureRequired`，UI 分别展示。 |
| 屏幕采集 | 桌面端有成熟采集管线，被控端持续生产帧。 | OHOS 通过 App native screen capture 推帧到 core，core 从 incoming frame cache 取帧。 | 已实现待复验 | 需要验证分辨率变化、旋转、后台、锁屏、权限撤销。 | 帧时间戳持续更新，重连后旧帧缓存清理。 |
| 编码 / 解码 | 官方多平台支持多编码路径和硬件能力。 | 当前重点是 VP8/VP9 + libvpx/libyuv 软解/编码依赖。 | 部分实现 | 硬编硬解、H.264/H.265、多编码协商完整性未对齐官方桌面端。 | 只声明真实可用 codec；VP8/VP9 端到端可工作。 |
| 文件传输 | 双向上传下载、目录浏览、创建、删除、覆盖确认、进度和错误。 | 已接入 `job-error/job-done/job-progress/folder-files/create-remote-dir/delete-remote-path/file-transfer-start` 等事件。 | 已实现待复验 | 事件接入不等于完整可用；需要 UI 和沙箱路径验证。 | 双向文件、目录、覆盖、删除、大文件、中文名、权限错误全部通过。 |
| 终端 | 打开、输入、resize、关闭，输出可靠回流。 | `session_open_terminal/session_send_terminal_input/session_resize_terminal/session_close_terminal` 要走 official `Session`，输出 base64。 | 已实现待复验 | 需要验证二进制/control bytes、resize、断线关闭。 | 终端输出通过 `dataBase64`，JSON 不被控制字符破坏。 |
| 聊天 | 文本消息发送/接收，签名与官方桥接保持一致。 | 四参 ABI 已要求对齐：peer_id/message_type/content/timestamp；旧一参只作 fallback。 | 已实现待复验 | 需要验证 App 侧实际调用参数位置，避免 content 读错。 | 新四参调用正常，旧一参兼容不破坏内容。 |
| 剪贴板 | 双向文本/可能的多格式剪贴板同步。 | 旧 mirror 不得保留 `false` stub，active bridge 要构造 Clipboard protobuf 发送。 | 已实现待复验 | 图片/文件等复杂剪贴板不承诺；文本需真测。 | 文本、中文、空字符串、双向同步通过。 |
| 远程光标 | 光标形状、位置、显示/隐藏与远端状态同步。 | 历史记录中 `set_cursor_data/set_cursor_id/set_cursor_position/set_display/main_set_cursor_position` 是高风险项。 | 待补齐/待复验 | 需要 official handler 回调到 bridge event，再到 ArkTS UI 展示。 | 光标形状和位置随远端变化，开关项真实生效。 |
| 多显示器 | 获取显示器列表，切换显示器，状态同步。 | `session_switch_display` 等命令存在并要求返回 bool。 | 已实现待复验 | 显示器列表、当前 display、失败原因需要真测。 | Windows 多显示器可切换，UI 状态同步。 |
| 菜单命令 | block-input、privacy-mode、switch-sides、record、voice、screenshot 等命令和状态。 | 部分命令已接入 bool 返回和事件回流。 | 部分实现/待复验 | 不能只发送命令，必须回读状态；`block-input` 等状态分支需权威来源。 | 每个菜单项有成功/失败/不支持状态，UI 不误报。 |
| 语音通话 | 呼叫、等待、接听、关闭、音频通道。 | started/waiting/incoming/closed 事件有补齐记录。 | 未测 | 事件存在不代表音频通道可用。 | 双端完整呼叫流程可用，失败有原因。 |
| 音频播放/采集 | 远端音频播放，本机麦克风/扬声器通道。 | `pull_audio_frames_json()` 空队列必须返回 `[]`。 | 未测 | 音频格式、播放、采集、权限、延迟都未验收。 | 有声输出/输入，断线恢复，空队列稳定返回 `[]`。 |
| 录制 | 开始/停止录制，状态回流，文件保存。 | `session_record_screen` 和 record 状态事件有补齐记录。 | 未测 | 保存路径、权限、文件生成、错误处理未验证。 | 能生成录制文件并正确反馈状态。 |
| 截图 | 请求截图，响应和保存。 | `handle_screenshot_resp` 有补事件回流记录。 | 未测 | 文件保存、权限、错误回流未验证。 | 截图文件生成，UI 收到成功/失败。 |
| LAN 发现 | 局域网发现、监听、刷新。 | `rendezvous_mediator_ohos.rs` 有独立适配和 LAN listening 修复记录。 | 已实现待复验 | 需要多网络环境测试。 | 同 LAN 可发现，对网络切换有恢复。 |
| IPv4/IPv6/relay | 混合地址族、直连候选、relay fallback。 | 有 IPv4/IPv6 混合连接修复记录。 | 已实现待复验 | 需要当前基线实测 IPv4-only、IPv6、relay。 | 地址族不匹配不会卡死，relay 可兜底。 |
| 后台/锁屏/生命周期 | 桌面端服务常驻，移动端处理前后台和权限。 | HarmonyOS 侧仍需 App 生命周期配合。 | 部分实现 | 后台、锁屏、权限撤销、进程回收策略未完整对齐。 | 前后台/锁屏场景行为明确，不误报服务 ready。 |
| 官方 FFI 覆盖 | Flutter FFI 暴露的大量能力可被 UI 调用。 | Harmony 使用专用 NAPI/C ABI，不是一比一复制。 | 部分实现 | 需要自动/人工生成 official FFI 对照表。 | 每次 RustDesk 升级都有函数级差异清单。 |
| 双架构构建 | 官方多平台构建稳定。 | Windows 双架构主构建，Linux arm64 手动构建。 | 已实现待持续验证 | Linux x86_64 尚未作为同等主路径；依赖缓存和 SDK secret 需维护。 | `core-*` 双资产发布；`core-linux-*` arm64 可手动产出。 |

## 当前已实现项

以下能力当前可以视为“已实现或已接入官方路径”，但其中部分仍需要持续回归测试：

| 已实现项 | 依据/边界 | 不能回退到 |
|----------|-----------|------------|
| official session 连接入口 | `connect_to_peer()` 走 official `Session` 并保存 active session。 | 旧假连接、旧网络 stub。 |
| 出站远控画面 | `on_rgba -> publish_real_video_frame -> video-frame`。 | 静态假帧、旧 latest frame 假状态。 |
| 出站远控输入桥接 | `send_mouse_input()` 等走 active session。 | 返回固定 false/true 的 stub。 |
| HarmonyOS 被控端画面传输 | 用户真机实测 Windows 端可见持续刷新画面。 | README 旧结论“屏幕共享不可用”。 |
| 文件传输事件桥接 | job/progress/folder/delete/create/start 事件已接入。 | 空回调、泛化 file-transfer 事件。 |
| 终端桥接 | terminal open/input/resize/close 调 official session，输出 base64。 | false stub、直接塞控制字符进 JSON。 |
| 聊天四参 ABI | C ABI/C++/d.ts 对齐 peer_id/message_type/content/timestamp。 | 只读 args[0] 的旧一参路径。 |
| 剪贴板文本路径 | active 和旧 mirror 都不能保留 false stub。 | mirror 覆盖 active 后退回 false。 |
| 部分菜单命令 bool 返回 | switch/record/voice/screenshot 等命令要求返回真实状态。 | 静默 true 或无 active session 也成功。 |
| 双架构 Windows 构建 | `core-*` 发布 arm64 和 x86_64 两个资产。 | 只发布半成品或空标签。 |
| Linux arm64 手动构建 | `core-linux-*` 使用 Linux SDK secret 手动发布。 | 复用 Windows SDK secret 或错误产物路径。 |

## 当前未实现 / 未验证 / 平台不支持项

| 项目 | 分类 | 当前处理方式 | 后续动作 |
|------|------|--------------|----------|
| HarmonyOS 被控端远程输入/操控 | 平台不支持 | UI/状态必须显示 unsupported，不作为发布阻塞项。 | 除非找到官方/企业级可用输入 API，否则不推进普通应用输入注入。 |
| `incomingReady` 精确定义 | 待补齐 | 不能代表完整被控能力。 | 拆分 `serviceReady/screenReady/inputReady/captureRequired`。 |
| 文件传输完整体验 | 已实现待复验 | 不宣称完整可用。 | 做双向文件、目录、覆盖、删除、大文件、中文名、权限错误测试。 |
| 远程光标 | 待补齐/待复验 | 不宣称完成。 | 补 handler event 和 ArkTS UI 展示。 |
| 菜单状态回读 | 部分实现 | 命令存在不等于状态可靠。 | 为每个菜单项补权威状态、失败原因和 UI 同步。 |
| 音频 | 未测 | 不宣称可用。 | 测播放、采集、格式、空队列、权限。 |
| 语音通话 | 未测 | 不宣称可用。 | 测呼叫/接听/关闭完整流程。 |
| 录制 | 未测 | 不宣称可用。 | 测状态、文件保存、权限、失败。 |
| 截图 | 未测 | 不宣称可用。 | 测响应、文件保存、权限、失败。 |
| 后台/锁屏/权限撤销 | 未测 | 不宣称稳定。 | 做系统生命周期专项测试。 |
| 官方 Flutter FFI 完整覆盖 | 待补齐 | 当前是 Harmony 专用 bridge。 | 生成函数级对照表并随上游升级更新。 |
| Linux x86_64 同等构建 | 未实现 | Linux 当前 arm64 手动路径为主。 | 需要补 x86_64 wrapper、target 分支、依赖构建逻辑和 release matrix。 |

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
7. 不能把平台不支持项包装成“暂未测试”。平台限制必须单独标识，避免后续误当 bug 反复追。

## 函数级对齐方法

后续做极限审计时，不要只看函数名是否存在。建议按下面顺序做：

1. 从 RustDesk 1.4.7 official `flutter_ffi.rs`、`Session`、`InvokeUiSession`、desktop server 相关 trait 收集官方能力。
2. 对照 Harmony active bridge：
   - `rustdesk-master/src/harmony_bridge/core.rs`
   - `native_rust_core/src/bridge_api.rs`
   - `cpp/rustdesk_bridge_abi.h`
   - `cpp/rustdesk_bridge_loader.cpp`
   - `cpp/types/librustdesk_bridge/index.d.ts`
3. 每个函数标记五个状态：
   - Rust 是否调用 official 实现。
   - C ABI 是否暴露。
   - C++ NAPI 是否参数读取正确。
   - d.ts 是否签名一致。
   - ArkTS UI 是否真实调用并处理返回值/事件。
4. 对每个能力补一个真实验收用例；没有用例的功能只能标为“已实现待复验”或“未测”。

## 极限对齐审计清单

### A. Official Session / FFI 对照

- [ ] 列出 RustDesk 1.4.7 official `Session`、`InvokeUiSession`、`flutter_ffi` 暴露能力。
- [ ] 对照 `harmony_bridge/core.rs`、`native_rust_core/src/bridge_api.rs`、`cpp/rustdesk_bridge_abi.h`、`cpp/rustdesk_bridge_loader.cpp`、`cpp/types/librustdesk_bridge/index.d.ts`。
- [ ] 每个函数标记：已实现、部分实现、未实现、平台不支持、未测。
- [ ] 所有 ArkTS 调用路径确认是否走 direct API 还是 generic option helper。
- [ ] 所有命令类 API 检查是否有真实 bool 返回和失败事件。

### B. 出站远控验收

- [ ] Windows/Linux/Android 远端各连一次。
- [ ] 自建服务器 key、relay、直连分别验证。
- [ ] 鼠标移动、左/右键、滚轮、键盘、Ctrl+Alt+Del 验证。
- [ ] 远程光标显示/隐藏验证。
- [ ] 多显示器切换验证。
- [ ] 断网重连、Wrong Password 后重试验证。
- [ ] 菜单项逐个验证状态回读。

### C. HarmonyOS 被控端验收

- [x] 画面传输：HarmonyOS 真机作为被控端，Windows 端能看到持续刷新画面。
- [ ] 画面持续 5 分钟以上无卡死、无 panic、无 fatal。
- [ ] 锁屏/息屏/后台/前台切换后的行为记录。
- [ ] relay 模式被控画面验证。
- [ ] 重连后首帧缓存清理验证。
- [ ] 输入/操控：当前平台不支持，UI 必须明确显示 unsupported，不给用户可控预期。
- [ ] `screenReady/inputReady/serviceReady/captureRequired` 状态拆分并展示。

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
- [ ] job_id 能把 start/progress/done/error 对上同一任务。

### E. 剪贴板 / 聊天 / 终端

- [ ] 聊天四参调用和旧一参 fallback 都不破坏内容。
- [ ] 剪贴板双向文本、中文、空字符串。
- [ ] 终端打开、输入、resize、关闭。
- [ ] 终端二进制/控制字符必须通过 `dataBase64`，不能破坏 JSON。
- [ ] 无 active session 时返回失败，不静默成功。

### F. 音频 / 语音 / 录制 / 截图

- [ ] `pull_audio_frames_json()` 空队列返回 `[]`。
- [ ] 远端音频播放。
- [ ] 语音呼叫完整状态流。
- [ ] 录制开始/停止/状态/文件保存。
- [ ] 截图请求/响应/保存/错误回流。
- [ ] 权限不足时给出明确失败事件。

### G. 构建 / 发布

- [ ] Windows 双架构 `core-*` 仍为主发布路径。
- [ ] Linux arm64 `core-linux-*` 可手动构建并验证产物大小。
- [ ] 每次发布前检查 `.a` 体积在 `100,000,000` 到 `250,000,000` bytes。
- [ ] 每次 HAP 打包后核对 CoreBuildInfo 的大小、mtime、hash。
- [ ] 不用 dev profile 产物；异常 568 MiB 级别产物一律拒绝。
- [ ] Release title/body/asset 命名保持英文，避免线上包标题混入中文。

## 当前优先级

1. 补 `screenReady/inputReady/serviceReady/captureRequired` 状态拆分，避免 `incomingReady` 含义过载。
2. 更新 App UI/状态模型，明确被控端“画面可用、输入不支持、其他未测”。
3. 做 HarmonyOS 被控端 5 分钟稳定性、relay、重连、锁屏/后台测试。
4. 做文件传输双向完整测试。
5. 做远程光标、菜单状态、剪贴板、终端复验。
6. 最后推进音频、语音、录制、截图。
7. 建立 official FFI 函数级自动/半自动对照清单，防止上游升级后能力倒退。

## 禁止回退项

- 不能把已经走 official `Session` 的函数退回 stub。
- 不能把 HarmonyOS 输入注入伪装成可用。
- 不能仅凭 native buffer 有帧就宣称完整被控端 ready。
- 不能仅凭符号导出就宣称文件传输/语音/截图/录制完成。
- 不能让旧 mirror core 覆盖 active bridge 的已修复逻辑。
- 不能把“平台不支持”写成“未测试”，否则后续会被误判成待修 bug。
- 不能让 C ABI、C++ NAPI、d.ts 和 ArkTS 调用签名不一致。
