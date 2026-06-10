@echo off
call "%~dp0_ohos-sdk-env.cmd" || exit /b 1

"%LLVM_BIN%\clang++.exe" ^
  --target=aarch64-linux-ohos ^
  --sysroot="%OHOS_SYSROOT%" ^
  -D__MUSL__ ^
  -stdlib=libc++ ^
  %*