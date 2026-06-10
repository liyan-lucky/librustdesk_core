@echo off
call "%~dp0_ohos-sdk-env.cmd" || exit /b 1

"%LLVM_BIN%\llvm-ar.exe" %*