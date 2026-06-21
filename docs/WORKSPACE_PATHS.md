# 工作区路径与 Core 构建测试规范

> 更新时间：2026-06-21 17:01（Europe/Berlin）  
> 最终规范复核：2026-06-21 23:23（Europe/Berlin）  

> 2026-06-22 00:30 最终状态：线上 `core-34` 两个 `.a` 与 `OpenRustdesk-Build-v0.33.6` HAP 保留在 `99_Temp\release_inspect` 作为不可替代的发布证据；HAP 已在 arm64 真机和 `127.0.0.1:5555` x86_64 虚拟机安装/冷启动通过。清理仍只允许触及下文明确归属本项目且可再生的目录，任何 APK 均不得删除。

> 2026-06-22 00:31 App 本地输出再次精简：删除 `.cxx`、unsigned HAP、source map 和 `pack.info` 共 `39,919,723` bytes，只保留 SHA256 `1D5C7395753D4E8F143FA051E0E931CCFB6C48FFEDA03A8DF91282DD007EC8D2` 的 signed HAP。该操作未触及 Core 线上资产、依赖缓存、签名材料、TabSSH 或任何 APK。

## 2026-06-21 23:23 强制统一规则

`F:\Visual_Studio_Code\99_Temp` 是多项目共享临时根，禁止整体删除、移动或全局按扩展名清理；任何目录中的 APK 一律保留。Core 只允许使用 `99_Temp\librustdesk_core`（target/cache/log）、`99_Temp\release_inspect\13_librustdesk_core`（线上资产复验）和 `99_Temp\rustdesk_core_backups`（仅保留最新 2 份）这些明确子目录。App 构建、stage、真机证据和备份由 App 文档列出的独立子目录负责，Core 清理不得触碰。工作区根 `_tmp_rustdesk_1_4_7_src` 只是历史上游 1.4.7 临时解压/对照树，不是第三仓库或构建输入，确认无独有修改后删除。

2026-06-22 00:36 实测：`_tmp_rustdesk_1_4_7_src` 不存在；前两次可计量 RustDesk 白名单清理释放 `1,348,092,177` bytes，最终审计后再次清理重建产物；共享根 3 个 APK 的数量、大小、SHA256 不变。Core 最新备份为 `99_Temp\rustdesk_core_backups\rustdesk_core_20260622_003605.zip`（`3,596,189` bytes，SHA256 `208346582AC4FAD62B20402DD256BC4519F33414969AE599F22AA5232773D949`），sidecar 与实算哈希一致；目录只保留最新 2 份 zip 及 `.sha256`。App 同期备份为 `rustdesk_harmonyos_20260622_003605.zip`（`1,440,526` bytes，SHA256 `D386142694D53E1E1154535818AB0573EEDE591AFE906242F84E14FA7D85E037`）。

统一验证为 100 轮：每 5 轮增量审计/文档检查点，每 10 轮全量构建/审计检查点；第 100 轮后冻结 Core/HAP 哈希并核对两架构大小、SHA256、导出符号、HAP CoreBuildInfo、真机 updateTime 和 hilog。任何 Core 源码或构建配置变化都必须生成新哈希并重跑最终证据链。一次性密码不得写入 Core 源码、日志、文档、截图、备份说明或提交说明。
> 本文与 App 仓库 `docs/WORKSPACE_PATHS.md` 保持一致，是 Core 项目的路径权威说明。

## 当前权威根目录

`%VSCODE_ROOT%` 表示包含 App、Core 和统一临时目录的工作区根目录。当前机器上它是：

```text
F:\Visual_Studio_Code
```

| 路径 | 用途 | 规则 |
| --- | --- | --- |
| `%VSCODE_ROOT%\13_librustdesk_core` | Core 源码仓库 | 只放 Rust/C++/TS bridge 源码、上游源码、patch、脚本、CI 和文档。 |
| `%VSCODE_ROOT%\11_Rustdesk_harmonyos` | App 仓库 | 消费 Core 构建产物并构建 HAP。 |
| `%VSCODE_ROOT%\99_Temp` | 唯一构建、测试、缓存、备份和临时证据根目录 | Core target、依赖构建缓存、日志、release 检查和备份都放这里。 |

废弃路径：

| 路径 | 状态 |
| --- | --- |
| `F:\99_Temp` / `\99_Temp` | 废弃的盘符根临时目录。不要再写入。 |
| `C:\99_Temp` | 废弃。不要用于本项目。 |
| `%VSCODE_ROOT%\_tmp_rustdesk_1_4_7_src` | 已删除的官方 RustDesk `1.4.7` tag 临时 clone。它不是当前 Core 改造源码；需要上游参考时重新从官方仓库临时 clone 到 `99_Temp`，不要放在工作区根。 |
| Core/App 仓库内 `.codex_*`、`target/` 大缓存、临时日志 | 只允许短时存在；交接前迁移或删除。 |

## 2026-06-21 清理结果

- App 仓库内 `.codex_*` 诊断文件、Core/Hvigor 大缓存和临时 HAP 输出已迁移/删除；最新 HAP/Core 产物已统一放入 `%VSCODE_ROOT%\99_Temp`。
- 废弃目录 `F:\99_Temp` 与旧散落备份 `%VSCODE_ROOT%\99_Temp\backups` 已删除。
- `%TEMP%` 中 RustDesk/Rundesk 截图、布局 JSON、HAP 签名解包目录和临时 Core build cache 已删除；保留的 0-byte `*.rustdesk` 是运行态 marker。
- 16:26 二次清理已完成：删除 `%VSCODE_ROOT%\_tmp_rustdesk_1_4_7_src`、`99_Temp\rustdesk_harmonyos_build\native_rust_core\target`、`99_Temp\rustdesk_harmonyos_build\windows_hap`、`99_Temp\rustdesk_harmonyos_build\rustdesk-1.4.7-clone`、旧 downloads/build、HAP build 的 `intermediates/cache/generated`。
- Core 仓库内 `.codeartsdoer/`、`rustdesk-master/target/`、`native_rust_core/target/`、根目录 `build_debug_*` / `build_env_*` / `cargo_build_*` 日志已删除。当前 Core ignored 保留项只应是 `entry/` 静态库副本、`rdev-fork/` OHOS 输入 fork 源码、`rustdesk-master/src/version.rs` 生成版本文件。
- App 仓库内 `.codeartsdoer/`、`.idea/`、`.hvigor/`、`oh_modules/`、`entry/oh_modules/`、`check_i18n.py`、`check_result.txt`、`entry/src/main/cpp/undefined_symbols.txt` 已删除。当前 App ignored 保留项只应是 Core junction、`entry/src/main/libs/`、`local.properties` 和 `signing/`。
- 2026-06-21 17:01 最终核验：工具重新生成的 App `.codeartsdoer/` 已再次删除；Core `rustdesk-master/libs/rdev/` 经确认只有空目录 `.github/` 与 `src/`，已删除。实际 OHOS rdev 源码保留在 `rdev-fork/`。

当前保留的关键产物：

| 产物 | 路径 | SHA256 |
| --- | --- | --- |
| arm64 Core archive | `%VSCODE_ROOT%\99_Temp\librustdesk_core\cargo_target\aarch64-unknown-linux-ohos\release\librustdesk_harmony_bridge.a` | `E4614BAE4EDB54F2C0A2CFECE96A2E99D558B6900693B2B3A9B08B8F3DCD5D5D` |
| x86_64 Core archive | `%VSCODE_ROOT%\99_Temp\librustdesk_core\cargo_target\x86_64-unknown-linux-ohos\release\librustdesk_harmony_bridge.a` | `DB0283F44EA5E5D09A23D1756929B171F28FF2A602D595941902A18ECE5F17DD` |
| Core 清理后备份 | `%VSCODE_ROOT%\99_Temp\rustdesk_core_backups\rustdesk_core_20260621_164050.zip` (`3,588,905` bytes) | `B64E5962551103380CF6DCDBDB1632124965DDF1B75FD47589F6982DBB0E85DA` |
| App 清理后备份 | `%VSCODE_ROOT%\99_Temp\rustdesk_harmonyos_backups\rustdesk_harmonyos_20260621_164050.zip` (`1,424,210` bytes) | `0ED94CEE63D8CDE9846B2EE3D6CFEA24BA67BAB1CB61F5668E6348CBDE3427CB` |

当前 `%VSCODE_ROOT%\99_Temp` 仅保留以下目录（2026-06-21 16:26 实测）：

| 子目录 | 大小 | 保留原因 |
| --- | ---: | --- |
| `rustdesk_harmonyos_build` | `6165.37 MB` | 仅保留 SDK/HMS/DevEco/vcpkg/external-src/tools/toolchains/patches 等依赖镜像和工具链；旧 target/HAP/clone/log 已删除。 |
| `librustdesk_core` | `268.40 MB` | 当前标准双架构 Core 产物和 manifest。 |
| `harmonyos_build` | `71.16 MB` | 当前标准 signed HAP 输出。 |
| `rustdesk_harmonyos_backups` | `2.72 MB` | App 最新 2 份备份及 `.sha256`。 |
| `harmonyos_cache` | `6.90 MB` | Hvigor/DevEco 缓存，可按需重建。 |
| `rustdesk_core_backups` | `6.84 MB` | Core 最新 2 份备份及 `.sha256`。 |
| `rustdesk_harmonyos_signing` | `0.02 MB` | 便携签名材料，必须保留。 |

## Core 构建输出规范

本地构建前统一设置：

```powershell
$env:VSCODE_ROOT = 'F:\Visual_Studio_Code'
$env:CARGO_TARGET_DIR = "$env:VSCODE_ROOT\99_Temp\librustdesk_core\cargo_target"
$env:RUSTDESK_CORE_BUILD_CACHE = "$env:VSCODE_ROOT\99_Temp\librustdesk_core\build_cache"
$env:RUSTDESK_CORE_BUILD_LOG_DIR = "$env:VSCODE_ROOT\99_Temp\librustdesk_core\build_logs"
```

推荐双架构构建：

```powershell
Set-Location "$env:VSCODE_ROOT\13_librustdesk_core"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build_native_bridge.ps1 -TargetTriple aarch64-unknown-linux-ohos -Profile release
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build_native_bridge.ps1 -TargetTriple x86_64-unknown-linux-ohos -Profile release
```

## `%VSCODE_ROOT%\99_Temp` 中 Core 相关目录

| 子目录 | 用途 | 清理规则 |
| --- | --- | --- |
| `librustdesk_core\cargo_target` | Cargo target | 可删除重建。 |
| `librustdesk_core\build_cache` | C/C++ 依赖构建缓存 | 可删除重建；排查依赖污染时优先清它。 |
| `librustdesk_core\build_logs` | 构建日志 | 只保留最近有效日志。 |
| `rustdesk_harmonyos_build` | 历史/辅助 SDK/HMS/DevEco mirror、vcpkg、外部源码、tools、toolchains、patches | 只保留依赖镜像和工具链。旧 `native_rust_core\target`、`windows_hap`、`rustdesk-1.4.7-clone`、downloads/build、日志和临时命令已删除；不要无确认整目录删除。 |
| `rustdesk_core_backups` | Core 仓库 zip 备份 | 只保留最新 2 份及 `.sha256`。 |

## 证据与隐私

- Release 资产必须检查两个 `.a` 同时存在、大小合理并记录 SHA256；任一架构失败不得发布空标签或半成品 Release。
- 构建日志不得保存一次性密码或设备隐私画面；临时截图和布局 dump 归纳后删除。
- 备份统一由 `scripts\backup_project.ps1` 生成到 `%VSCODE_ROOT%\99_Temp\rustdesk_core_backups`，不要再使用散落 `backups` 目录。
