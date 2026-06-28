# librustdesk_core 文档索引

> 核心项目文档入口。当前主状态以本文件、根目录 `README.md` 和 `OFFICIAL_CORE_GAP.md` 为准。历史测试包编号、历史 SHA、历史包体积和旧 Release 记录不再作为当前状态展示。

## 当前状态摘要

- 本项目为第三方非官方 HarmonyOS / OpenHarmony 适配项目，不代表上游项目官方发布、认可、赞助或背书。
- 当前发布标签统一使用 `core-001`、`core-002`、`core-003` 形式。
- Windows 和 Linux 共用同一套版本编号。
- 构建启动后立即预留版本号；失败也保留标签占号。
- 只有 arm64 和 x86_64 两个产物都完整生成并校验通过，才创建 Release 并上传正式包。
- Linux 可由 main 更新自动触发，也可手动触发；Windows 保持手动触发。
- Release 说明只更新介绍内容，不改标签和 Release 名称。
- HarmonyOS 被控端画面传输已由真机实测跑通。
- HarmonyOS 被控端远程输入/操控当前按平台不支持处理，不作为发布阻塞项，也不能在 UI 或状态中宣称支持。
- 文件传输、音频、语音、录制、截图、远程光标、完整菜单状态等仍按未逐项端到端验证处理。

## 推荐阅读顺序

1. `README.md`：项目当前状态、发布规则、合规说明。
2. `LICENSE_NOTICE.md`：许可证入口、源码来源、再分发说明和非官方声明。
3. `OFFICIAL_CORE_GAP.md`：上游核心能力对齐、未实现项、平台限制和验收边界。
4. `CORE.md`：核心架构、可复现编译、桥接函数和构建问题。
5. `CONNECTION_DEBUG_LOG.md`：连接问题逐轮排查记录。
6. `WORKSPACE_PATHS.md`：Core 构建、测试、备份路径规范。
7. `THIRD_PARTY_NOTICES.md`：第三方组件和合规提示。

## 文档列表

| 文件 | 说明 |
|------|------|
| `LICENSE_NOTICE.md` | 许可证入口、源码来源、再分发说明和非官方声明 |
| `COPYING` | 根目录许可证入口，指向上游许可证文本和合规说明 |
| `NOTICE` | 项目非官方声明和源码来源说明 |
| `CORE.md` | 核心架构、可复现编译、桥接函数、CMake 链接、编译问题 |
| `OFFICIAL_CORE_GAP.md` | 上游核心对齐审计；列出已接通、未测、平台不支持和待补齐项 |
| `THIRD_PARTY_NOTICES.md` | 第三方源码、依赖和再分发前的合规提示 |
| `WORKSPACE_PATHS.md` | Core 构建/测试/备份路径规范；必须与 App 仓库同名文档保持一致 |
| `LESSONS_LEARNED.md` | 经验教训和易复发构建问题 |
| `BUILD_ARCHIVE.md` | 历史构建、脚本、Ubuntu 路径和早期会话归档 |
| `CONNECTION_DEBUG_LOG.md` | 连接问题逐轮排查记录 |
| `UBUNTU_CROSS_COMPILE_GUIDE.md` | Ubuntu 交叉编译指南 |
| `SESSION3_SUMMARY.md` | 会话 3 总结 |
| `WINDOWS_SERVICE_OPTIMIZATION.md` | Windows 服务优化 |
| `FUNCTION_LOGIC_AUDIT_2026-06-05.md` | 功能逻辑审计（6 月 5 日） |
| `FUNCTION_LOGIC_AUDIT_2026-06-06.md` | 功能逻辑审计（6 月 6 日） |
| `OHOS_CODE_MAP.md` | OHOS 专属代码分布说明，便于更新上游源码 |

## 核心修改流程

1. 在本项目中修改 Rust/C++/TS 桥接代码。
2. 如需要，运行代码生成脚本：`node scripts/regenerate_all.js`。
3. 本地验证编译：
   - Windows：`powershell -File scripts/build_native_bridge.ps1`
   - Linux：`./scripts/build_native_bridge.sh <target-triple> release`
4. 推送到远端。
5. Linux 自动触发器或手动 workflow 启动构建。
6. 构建启动后预留 `core-XXX` 标签。
7. arm64 和 x86_64 两个产物都成功并通过校验后才创建 Release。
8. 下载 Release 产物，放入 HAP 项目对应 ABI 目录。
9. 如桥接层有更新，同步 `cpp/` 和 `cpp/types/` 到 HAP 项目。

## CI/CD 在线构建

| 工作流 | 环境 | 触发方式 | 输出 | 说明 |
|--------|------|----------|------|------|
| `build-core-windows.yml` | windows-2022 | 手动 | `librustdesk_core.a` + `librustdesk_core_x86_64.a` | Windows 双架构构建 |
| `build-core-linux.yml` | ubuntu-22.04 | 手动，或由自动触发器调用 | `librustdesk_core.a` + `librustdesk_core_x86_64.a` | Linux 双架构构建，需 `OHOS_SDK_LINUX_ZIP_URL` |
| `auto-linux-core-build.yml` | ubuntu-latest | main 更新后自动 | 无直接产物 | 自动触发 Linux 构建 |
| `update-core-release-notes.yml` | ubuntu-latest | 构建成功后自动 | 更新 Release 说明 | 只改介绍内容，不改标签和名称 |
| `cleanup-releases.yml` | ubuntu-latest | 手动 | 删除 Release、core 标签和旧 Actions 运行记录 | 用于重置编号和清理历史记录 |

## 发布规则

- 当前只使用统一 `core-XXX` 标签。
- 不再使用 `core-linux-*`。
- 构建失败仍占用编号，但不创建 Release，不上传正式包。
- Release 说明由 `.github/scripts/write-core-release-notes.sh` 生成。
- `core-001` 使用首版详细介绍，后续版本使用更新说明模板。

## 合规与品牌边界

- 不使用“官方项目”“官方发布”“官方授权”等暗示上游背书的表述。
- 文档中“上游”仅表示源码来源和兼容目标。
- 发布静态库时应保留源码、构建脚本、补丁、第三方声明和许可证说明。
- 许可入口见 `LICENSE_NOTICE.md` 和 `COPYING`。
- 上游许可证文本保留在 `rustdesk-master/LICENCE`。
- 第三方说明见 `NOTICE` 和 `docs/THIRD_PARTY_NOTICES.md`。

## HAP 项目（11_Rustdesk_harmonyos）文档

HAP 项目文档聚焦应用层：

| 文件 | 说明 |
|------|------|
| `AGENT_MEMORY.md` | AI 助手工作规则、经验库、用户偏好 |
| `CORE.md` | 核心状态、HAP 构建安装、运行验证清单 |
| `docs/AGENT_HANDOFF.md` | 应用层交接摘要 |

HAP 项目中的核心版本、包体积和运行状态应引用当前 Release 和本仓库最新文档，不再引用旧测试包编号作为当前状态。
