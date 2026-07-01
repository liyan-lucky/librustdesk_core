# librustdesk_core 文档索引

> 核心项目文档入口。当前主状态以 `docs/CURRENT_STATUS.md`、根目录 `README.md` 和 `OFFICIAL_CORE_GAP.md` 为准。历史测试包编号、历史 SHA、历史包体积和旧 Release 记录不再作为当前状态展示。

## 当前状态入口

- `CURRENT_STATUS.md`：当前仓库事实、能力边界、目录职责、分支/备份策略和 CI/CD 状态。
- `README.md`：项目定位、构建、发布产物、合规说明。
- `OFFICIAL_CORE_GAP.md`：上游核心能力对齐、未实现项、平台限制和验收边界。

## 当前状态摘要

- 本项目为第三方非官方 HarmonyOS / OpenHarmony 适配项目，不代表上游项目官方发布、认可、赞助或背书。
- 当前发布标签统一使用 `core-001`、`core-002`、`core-003` 形式。
- Windows 和 Linux 共用同一套版本编号。
- 构建启动后立即预留版本号；失败也保留标签占号。
- 只有 arm64 和 x86_64 两个产物都完整生成并校验通过，才创建 Release 并上传正式包。
- Linux 可由 `main` 更新自动触发，也可手动触发；Windows 保持手动触发。
- `backup` 是 `main` 的快照备份分支，可通过 `.github/workflows/force-backup-main.yml` 手动输入 `YES` 强制覆盖。
- HarmonyOS 被控端画面传输已由真机实测跑通。
- HarmonyOS 被控端远程输入/操控当前按平台不支持处理，不作为发布阻塞项，也不能在 UI 或状态中宣称支持。
- 文件传输、音频、语音、录制、截图、远程光标、完整菜单状态等仍按未逐项端到端验证处理。

## 推荐阅读顺序

1. `CURRENT_STATUS.md`
2. `README.md`
3. `LICENSE_NOTICE.md`
4. `OFFICIAL_CORE_GAP.md`
5. `CORE.md`
6. `CONNECTION_DEBUG_LOG.md`
7. `WORKSPACE_PATHS.md`
8. `THIRD_PARTY_NOTICES.md`

## 主要文档列表

| 文件 | 说明 |
|------|------|
| `CURRENT_STATUS.md` | 当前仓库状态、分支/备份策略、CI/CD 和目录职责 |
| `LICENSE_NOTICE.md` | 许可证入口、源码来源、再分发说明和非官方声明 |
| `COPYING` | 根目录许可证入口，指向上游许可证文本和合规说明 |
| `NOTICE` | 项目非官方声明和源码来源说明 |
| `CORE.md` | 核心架构、可复现编译、桥接函数、CMake 链接、编译问题 |
| `OFFICIAL_CORE_GAP.md` | 上游核心对齐审计；列出已接通、未测、平台不支持和待补齐项 |
| `THIRD_PARTY_NOTICES.md` | 第三方源码、依赖和再分发前的合规提示 |
| `WORKSPACE_PATHS.md` | Core 构建/测试/备份路径规范 |
| `LESSONS_LEARNED.md` | 经验教训和易复发构建问题 |
| `BUILD_ARCHIVE.md` | 历史构建、脚本、Ubuntu 路径和早期会话归档 |
| `CONNECTION_DEBUG_LOG.md` | 连接问题逐轮排查记录 |
| `UBUNTU_CROSS_COMPILE_GUIDE.md` | Ubuntu 交叉编译指南 |
| `OHOS_CODE_MAP.md` | OHOS 专属代码分布说明，便于更新上游源码 |

## CI/CD 在线构建

| 工作流 | 触发方式 | 说明 |
|--------|----------|------|
| `build-core-windows.yml` | 手动 | Windows 双架构构建 |
| `build-core-linux.yml` | 手动或自动触发器调用 | Linux 双架构构建，需 `OHOS_SDK_LINUX_ZIP_URL` |
| `auto-linux-core-build.yml` | `main` 更新后自动 | 自动触发 Linux 构建 |
| `update-core-release-notes.yml` | 构建成功后自动 | 只更新 Release 说明，不改标签和名称 |
| `cleanup-releases.yml` | 手动 | 清理 Release、core 标签和旧 Actions 运行记录 |
| `force-backup-main.yml` | 手动 | 输入 `YES` 后强制覆盖 `backup` 分支 |

## 文档维护要求

当前能力、构建、Release、分支、备份、目录职责或合规边界变化时，必须同步更新 `CURRENT_STATUS.md`、根 README 和相关专项文档。
