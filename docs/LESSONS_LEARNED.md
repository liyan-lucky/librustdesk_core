# 经验教训

> 记录容易复发的构建、发布和排查问题。新增经验时优先写清楚：现象、根因、修复、以后如何避免。

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
