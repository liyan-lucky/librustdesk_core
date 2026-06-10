@echo off
setlocal EnableExtensions

if not "%RUSTDESK_HARMONY_HOST_SDK%"=="" (
  set "OHOS_ROOT=%RUSTDESK_HARMONY_HOST_SDK%"
) else if not "%OHOS_SDK_HOME%"=="" (
  set "OHOS_ROOT=%OHOS_SDK_HOME%"
) else if not "%OHOS_NDK_HOME%"=="" (
  set "OHOS_ROOT=%OHOS_NDK_HOME%"
) else (
  echo OHOS SDK not found. Set RUSTDESK_HARMONY_HOST_SDK or OHOS_SDK_HOME.
  exit /b 1
)

if exist "%OHOS_ROOT%\native\llvm\bin\clang.exe" (
  set "LLVM_BIN=%OHOS_ROOT%\native\llvm\bin"
  set "OHOS_SYSROOT=%OHOS_ROOT%\native\sysroot"
) else (
  echo Invalid OHOS SDK path: %OHOS_ROOT%
  echo Expected: %OHOS_ROOT%\native\llvm\bin\clang.exe
  exit /b 1
)

if not exist "%LLVM_BIN%\llvm-ar.exe" (
  echo llvm-ar.exe not found under %LLVM_BIN%
  exit /b 1
)

if not exist "%OHOS_SYSROOT%" (
  echo sysroot not found: %OHOS_SYSROOT%
  exit /b 1
)

endlocal & (
  set "OHOS_ROOT=%OHOS_ROOT%"
  set "LLVM_BIN=%LLVM_BIN%"
  set "OHOS_SYSROOT=%OHOS_SYSROOT%"
  set "PATH=%LLVM_BIN%;%PATH%"
)

exit /b 0