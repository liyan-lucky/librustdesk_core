param(
  [string]$TargetTriple = "aarch64-unknown-linux-ohos",
  [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"

# Create log file
$logFile = Join-Path $PSScriptRoot "..\build_debug_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
$cargoLogFile = Join-Path $PSScriptRoot "..\cargo_build_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
$envLogFile = Join-Path $PSScriptRoot "..\build_env_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"

function Write-Log {
  param([string]$Message)
  $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
  $logMessage = "[$timestamp] $Message"
  Write-Host $logMessage
  Add-Content -Path $logFile -Value $logMessage -Force
}

Write-Log "=== Build Native Bridge Started ==="
Write-Log "Target Triple: $TargetTriple"
Write-Log "Profile: $Profile"
Write-Log "Script log will be saved to: $logFile"
Write-Log "Cargo log will be saved to: $cargoLogFile"
Write-Log "Environment log will be saved to: $envLogFile"

if ($TargetTriple -ne "aarch64-unknown-linux-ohos") {
  throw "Unsupported target triple: $TargetTriple. Current HarmonyOS package ABI is arm64-v8a only."
}

$projectRoot = Split-Path -Parent $PSScriptRoot
Write-Log "Project Root: $projectRoot"

$nativeCoreDir = Join-Path $projectRoot "native_rust_core"
$buildRoot = if ($env:RUSTDESK_HARMONY_BUILD_DIR) {
  $env:RUSTDESK_HARMONY_BUILD_DIR
} else {
  [System.IO.Path]::GetFullPath((Join-Path $projectRoot "..\99_Temp\rustdesk_harmonyos_build"))
}
Write-Log "Build Root: $buildRoot"
Write-Log "Native Core Dir: $nativeCoreDir"

$cargoTargetDir = if ($env:CARGO_TARGET_DIR) {
  $env:CARGO_TARGET_DIR
} else {
  Join-Path $buildRoot "native_rust_core\target"
}
Write-Log "Cargo Target Dir: $cargoTargetDir"

$outputDir = Join-Path $cargoTargetDir "harmony"
$linkerScript = Join-Path $PSScriptRoot "$TargetTriple-clang.cmd"
$cxxScript = if ($TargetTriple -eq "aarch64-unknown-linux-ohos") {
  Join-Path $PSScriptRoot "$TargetTriple-clang++.cmd"
} else {
  $linkerScript
}
$llvmArScript = Join-Path $PSScriptRoot "ohos-llvm-ar.cmd"
$ohosEnvScript = Join-Path $PSScriptRoot "_ohos-sdk-env.cmd"
$localProperties = Join-Path $projectRoot "local.properties"
$hostSdkMirrorDir = Join-Path $buildRoot "deveco-sdk"
$cargoExe = $null
$rustupExe = $null
$vcvarsScript = $null
$msysBashExe = $null
$msysPerlExe = $null
$msysBinDir = $null
$cargoTargetKey = $TargetTriple.ToUpper().Replace('-', '_')
$targetEnvKey = $TargetTriple.ToLower().Replace('-', '_').Replace('.', '_')
$bindgenTarget = switch ($TargetTriple) {
  "aarch64-unknown-linux-ohos" { "aarch64-linux-ohos" }
  default { throw "Unsupported target triple: $TargetTriple" }
}
$sysrootIncludeDir = switch ($TargetTriple) {
  "aarch64-unknown-linux-ohos" { "aarch64-linux-ohos" }
  default { throw "Unsupported target triple: $TargetTriple" }
}
$configureHost = switch ($TargetTriple) {
  "aarch64-unknown-linux-ohos" { "aarch64-unknown-linux-gnu" }
  default { throw "Unsupported target triple: $TargetTriple" }
}
$vcpkgRoot = if ($env:VCPKG_ROOT) {
  $env:VCPKG_ROOT
} else {
  Join-Path $buildRoot "vcpkg"
}
$vcpkgInstalledRoot = if ($env:VCPKG_INSTALLED_ROOT) {
  $env:VCPKG_INSTALLED_ROOT
} else {
  Join-Path $vcpkgRoot "installed"
}

function Convert-ToForwardSlashPath {
  param([string]$Path)

  return ([System.IO.Path]::GetFullPath($Path) -replace '\\', '/')
}

function Convert-ToMsysPath {
  param([string]$Path)

  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $forwardPath = $fullPath -replace '\\', '/'
  if ($forwardPath -match '^([A-Za-z]):/(.*)$') {
    return "/$($matches[1].ToLowerInvariant())/$($matches[2])"
  }

  return $forwardPath
}

function Get-LocalPropertyValue {
  param(
    [string]$FilePath,
    [string]$Key
  )

  if (-not (Test-Path $FilePath)) {
    return $null
  }

  foreach ($line in Get-Content -Path $FilePath) {
    if ($line -like "$Key=*") {
      return $line.Substring($Key.Length + 1).Replace('\\', '\')
    }
  }

  return $null
}

function Get-SdkProbePaths {
  param([string]$RootPath)

  if ([string]::IsNullOrWhiteSpace($RootPath)) {
    return @()
  }

  $fullRoot = [System.IO.Path]::GetFullPath($RootPath)
  return @(
    $fullRoot,
    (Join-Path $fullRoot "default\openharmony"),
    (Join-Path $fullRoot "openharmony"),
    (Join-Path $fullRoot "sdk\default\openharmony")
  )
}

function Resolve-HostSdkDirectory {
  param(
    [string]$BuildRoot,
    [string]$LocalPropertiesFile
  )

  Write-Log "Resolving Host SDK Directory..."
  $candidates = New-Object System.Collections.Generic.List[string]
  foreach ($candidate in @(
    $env:RUSTDESK_HARMONY_HOST_SDK,
    $env:OHOS_NDK_HOME,
    $env:OHOS_SDK_HOME,
    (Join-Path $BuildRoot "deveco-sdk"),
    (Join-Path $BuildRoot "ohos-sdk"),
    (Join-Path $BuildRoot ".ohos-sdk"),
    (Join-Path $BuildRoot "tools\openharmony-sdk"),
    "C:\Program Files\Huawei\DevEco Studio\sdk\default\openharmony"
  )) {
    if (-not [string]::IsNullOrWhiteSpace($candidate)) {
      $candidates.Add($candidate)
      Write-Log "  Candidate: $candidate"
    }
  }

  $sdkFromProperties = Get-LocalPropertyValue -FilePath $LocalPropertiesFile -Key "sdk.dir"
  if ($sdkFromProperties) {
    $candidates.Add($sdkFromProperties)
    Write-Log "  Candidate from properties: $sdkFromProperties"
  }

  foreach ($candidate in $candidates) {
    foreach ($probePath in Get-SdkProbePaths -RootPath $candidate) {
      $clangExe = Join-Path $probePath "native\llvm\bin\clang.exe"
      $llvmArExe = Join-Path $probePath "native\llvm\bin\llvm-ar.exe"
      $sysrootDir = Join-Path $probePath "native\sysroot"
      Write-Log "  Checking: $probePath"
      Write-Log "    clang.exe exists: $(Test-Path $clangExe)"
      Write-Log "    llvm-ar.exe exists: $(Test-Path $llvmArExe)"
      Write-Log "    sysroot exists: $(Test-Path $sysrootDir)"
      if ((Test-Path $clangExe) -and (Test-Path $llvmArExe) -and (Test-Path $sysrootDir)) {
        $resolvedPath = [System.IO.Path]::GetFullPath($probePath)
        Write-Log "  Found SDK at: $resolvedPath"
        return $resolvedPath
      }
    }
  }

  Write-Log "WARNING: SDK not found in any candidate paths"
  return $null
}

function Ensure-NoSpaceSdkMirror {
  param(
    [string]$SdkDirectory,
    [string]$MirrorDirectory
  )

  if (-not $SdkDirectory) {
    return $null
  }

  $resolvedSdkDirectory = [System.IO.Path]::GetFullPath($SdkDirectory)
  $resolvedMirrorDirectory = [System.IO.Path]::GetFullPath($MirrorDirectory)

  if ($resolvedSdkDirectory -ieq $resolvedMirrorDirectory) {
    return $resolvedSdkDirectory
  }

  if ($resolvedSdkDirectory -notmatch '\s') {
    return $resolvedSdkDirectory
  }

  if (Test-Path $resolvedMirrorDirectory) {
    $mirrorItem = Get-Item -LiteralPath $resolvedMirrorDirectory -ErrorAction SilentlyContinue
    $mirrorTargets = @()
    if ($mirrorItem -and $mirrorItem.Target) {
      $mirrorTargets = @($mirrorItem.Target) | ForEach-Object {
        [System.IO.Path]::GetFullPath($_)
      }
    }
    if ($mirrorTargets -contains $resolvedSdkDirectory) {
      return $resolvedMirrorDirectory
    }
    Remove-Item -LiteralPath $resolvedMirrorDirectory -Recurse -Force
  }

  New-Item -ItemType Junction -Path $resolvedMirrorDirectory -Target $resolvedSdkDirectory | Out-Null
  return $resolvedMirrorDirectory
}

function Resolve-MsysSetupToolPath {
  param([string]$MsysPath)

  $wrappers = New-Object System.Collections.Generic.List[string]
  $pathWrapper = Get-Command "msys2.cmd" -ErrorAction SilentlyContinue
  if ($pathWrapper) {
    $wrappers.Add($pathWrapper.Source)
  }
  if ($env:RUNNER_TEMP) {
    $wrappers.Add((Join-Path $env:RUNNER_TEMP "setup-msys2\msys2.cmd"))
  }

  foreach ($wrapper in ($wrappers | Where-Object { $_ -and (Test-Path $_) } | Select-Object -Unique)) {
    try {
      $result = & $wrapper -c "cygpath -w '$MsysPath'" 2>$null
      if ($LASTEXITCODE -eq 0 -and $result) {
        $resolved = ($result | Select-Object -First 1).Trim()
        if ($resolved -and (Test-Path $resolved)) {
          return [System.IO.Path]::GetFullPath($resolved)
        }
      }
    } catch {
      Write-Log "  MSYS2 wrapper probe failed for $MsysPath via $wrapper : $($_.Exception.Message)"
    }
  }

  return $null
}

function Convert-MsysPathToWindowsPath {
  param(
    [string]$MsysBashExe,
    [string]$MsysPath
  )

  if ([string]::IsNullOrWhiteSpace($MsysBashExe) -or -not (Test-Path $MsysBashExe)) {
    return $null
  }

  try {
    $result = & $MsysBashExe -lc "cygpath -w '$MsysPath'" 2>$null
    if ($LASTEXITCODE -eq 0 -and $result) {
      $resolved = ($result | Select-Object -First 1).Trim()
      if ($resolved) {
        return [System.IO.Path]::GetFullPath($resolved)
      }
    }
  } catch {
    Write-Log "  MSYS2 cygpath probe failed for $MsysPath via $MsysBashExe : $($_.Exception.Message)"
  }

  return $null
}

function Resolve-MsysTool {
  param(
    [string[]]$Candidates,
    [string]$Description
  )

  foreach ($candidate in $Candidates) {
    if ($candidate -and (Test-Path $candidate)) {
      Write-Log "Found $Description at: $candidate"
      return [System.IO.Path]::GetFullPath($candidate)
    }
  }

  throw "$Description was not found. Install MSYS2 and make sure the tool exists in one of: $($Candidates -join ', ')"
}

function Resolve-OhosLibcxxIncludeDirectory {
  param(
    [string]$SdkDirectory,
    [string]$MsysBashExe,
    [switch]$Required
  )

  $sdkLlvmRoot = Join-Path $SdkDirectory "native\llvm"
  $candidateList = New-Object System.Collections.Generic.List[string]
  foreach ($candidate in @(
    $env:RUSTDESK_HARMONY_LIBCXX_INCLUDE,
    $env:OHOS_LIBCXX_INCLUDE,
    (Join-Path $sdkLlvmRoot "include\libcxx-ohos\include\c++\v1"),
    (Join-Path $sdkLlvmRoot "include\c++\v1"),
    "C:\msys64\clang64\include\c++\v1",
    "C:\msys64\mingw64\include\c++\v1",
    "C:\Program Files\LLVM\include\c++\v1",
    "C:\Program Files (x86)\LLVM\include\c++\v1"
  )) {
    if (-not [string]::IsNullOrWhiteSpace($candidate)) {
      $candidateList.Add($candidate)
    }
  }

  $candidates = $candidateList | Select-Object -Unique

  foreach ($candidate in $candidates) {
    if (Test-Path (Join-Path $candidate "cstdint")) {
      return [System.IO.Path]::GetFullPath($candidate)
    }
  }

  $searchRoots = @(
    $sdkLlvmRoot,
    "C:\Program Files\LLVM",
    "C:\Program Files (x86)\LLVM"
  ) | Where-Object {
    -not [string]::IsNullOrWhiteSpace($_) -and (Test-Path $_)
  }

  foreach ($searchRoot in $searchRoots) {
    $discovered = Get-ChildItem -Path $searchRoot -Recurse -Filter "cstdint" -File -ErrorAction SilentlyContinue |
      Where-Object {
        ($_.FullName -replace '\\', '/') -match '/include/.*/c\+\+/v1/cstdint$'
      } |
      Select-Object -First 1
    if ($discovered) {
      return [System.IO.Path]::GetFullPath($discovered.DirectoryName)
    }
  }

  $checked = $candidates | ForEach-Object {
    "$_ (cstdint exists: $(Test-Path (Join-Path $_ 'cstdint')))"
  }
  if ($Required) {
    throw "OpenHarmony SDK libc++ include directory was not found. Checked: $($checked -join '; ')"
  }

  Write-Log "  libc++ Include: not found. Checked: $($checked -join '; ')"
  return $null
}

function Resolve-LibsodiumCrateDirectory {
  $cargoRegistrySrcRoot = Join-Path $env:USERPROFILE ".cargo\registry\src"
  if (-not (Test-Path $cargoRegistrySrcRoot)) {
    throw "Cargo registry sources were not found under $cargoRegistrySrcRoot."
  }

  foreach ($registryRoot in Get-ChildItem -Path $cargoRegistrySrcRoot -Directory) {
    $candidate = Join-Path $registryRoot.FullName "libsodium-sys-0.2.7"
    if (Test-Path (Join-Path $candidate "libsodium\configure")) {
      return $candidate
    }
  }

  throw "libsodium-sys-0.2.7 source was not found in the Cargo registry. Run a normal cargo build once to download it."
}

function Resolve-LibsodiumHostImportLibrary {
  $crateDirectory = Resolve-LibsodiumCrateDirectory
  $hostArch = if ([Environment]::Is64BitProcess) { "x64" } else { "Win32" }
  $candidates = @(
    (Join-Path $crateDirectory "msvc\$hostArch\Release\v143\libsodium.lib"),
    (Join-Path $crateDirectory "msvc\$hostArch\Release\v142\libsodium.lib")
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return [System.IO.Path]::GetFullPath($candidate)
    }
  }

  throw "A host libsodium import library was not found under $crateDirectory\msvc\$hostArch\Release."
}

function Stage-LibsodiumHostImportLibrary {
  param([string]$LibDirectory)

  $hostImportLibrary = Resolve-LibsodiumHostImportLibrary
  New-Item -ItemType Directory -Path $LibDirectory -Force | Out-Null
  Copy-Item -LiteralPath $hostImportLibrary -Destination (Join-Path $LibDirectory "libsodium.lib") -Force
}

function Get-BuildJobCount {
  $jobs = 4
  $parsedJobs = 0
  if ([int]::TryParse($env:NUMBER_OF_PROCESSORS, [ref]$parsedJobs) -and $parsedJobs -gt 0) {
    $jobs = $parsedJobs
  }

  return $jobs
}

function Assert-PathInsideRoot {
  param(
    [string]$Path,
    [string]$RootPath,
    [string]$Description
  )

  $trimChars = [char[]]@('\', '/')
  $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd($trimChars)
  $fullRoot = [System.IO.Path]::GetFullPath($RootPath).TrimEnd($trimChars)
  $comparison = [System.StringComparison]::OrdinalIgnoreCase
  if (-not ($fullPath.Equals($fullRoot, $comparison) -or $fullPath.StartsWith("$fullRoot\", $comparison) -or $fullPath.StartsWith("$fullRoot/", $comparison))) {
    throw "$Description is outside the expected root. Path: $fullPath Root: $fullRoot"
  }
}

function Remove-DirectoryInsideRoot {
  param(
    [string]$Path,
    [string]$RootPath,
    [string]$Description
  )

  if (-not (Test-Path $Path)) {
    return
  }

  Assert-PathInsideRoot -Path $Path -RootPath $RootPath -Description $Description
  Remove-Item -LiteralPath $Path -Recurse -Force
}

function Ensure-ArchiveSource {
  param(
    [string]$Name,
    [string]$Url,
    [string]$ArchivePath,
    [string]$SourceDirectory,
    [string]$RequiredRelativePath,
    [string]$BuildRoot
  )

  $requiredPath = Join-Path $SourceDirectory $RequiredRelativePath
  if (Test-Path $requiredPath) {
    Write-Log "$Name source already available at: $SourceDirectory"
    return
  }

  Remove-DirectoryInsideRoot -Path $SourceDirectory -RootPath $BuildRoot -Description "$Name source directory"
  New-Item -ItemType Directory -Path $SourceDirectory -Force | Out-Null
  New-Item -ItemType Directory -Path (Split-Path -Parent $ArchivePath) -Force | Out-Null

  if (-not (Test-Path $ArchivePath)) {
    Write-Log "Downloading $Name from: $Url"
    Invoke-WebRequest -Uri $Url -OutFile $ArchivePath
  } else {
    Write-Log "$Name archive already available at: $ArchivePath"
  }

  Write-Log "Extracting $Name to: $SourceDirectory"
  & tar -xzf $ArchivePath -C $SourceDirectory --strip-components=1
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to extract $Name archive: $ArchivePath"
  }

  if (-not (Test-Path $requiredPath)) {
    throw "$Name source extraction completed, but required file was not found: $requiredPath"
  }
}

function Ensure-LibvpxStaticLibrary {
  param(
    [string]$BuildRoot,
    [string]$SdkDirectory,
    [string]$MsysBashExe,
    [string]$VcpkgInstalledRoot,
    [string]$BindgenTarget,
    [string]$SysrootIncludeDir
  )

  $triplet = "arm64-linux"
  $installRoot = Join-Path $VcpkgInstalledRoot $triplet
  $includeDir = Join-Path $installRoot "include"
  $libDir = Join-Path $installRoot "lib"
  $requiredHeaders = @(
    "vpx\vpx_decoder.h",
    "vpx\vp8cx.h",
    "vpx\vpx_encoder.h"
  ) | ForEach-Object {
    Join-Path $includeDir $_
  }
  $finalLib = Join-Path $libDir "libvpx.a"
  $missingHeaders = $requiredHeaders | Where-Object { -not (Test-Path $_) }
  if (($missingHeaders.Count -eq 0) -and (Test-Path $finalLib)) {
    Write-Log "libvpx already available at: $finalLib"
    return
  }

  $version = "1.15.2"
  $sourceDirectory = Join-Path $BuildRoot "external-src\libvpx-$version"
  $archivePath = Join-Path $BuildRoot "downloads\libvpx-$version.tar.gz"
  Ensure-ArchiveSource `
    -Name "libvpx $version" `
    -Url "https://github.com/webmproject/libvpx/archive/refs/tags/v$version.tar.gz" `
    -ArchivePath $archivePath `
    -SourceDirectory $sourceDirectory `
    -RequiredRelativePath "configure" `
    -BuildRoot $BuildRoot

  $jobs = Get-BuildJobCount
  New-Item -ItemType Directory -Path $includeDir, $libDir -Force | Out-Null

  $sdkLlvmBin = Join-Path $SdkDirectory "native\llvm\bin"
  $sdkLlvmBinMsys = Convert-ToMsysPath $sdkLlvmBin
  $sdkSysrootMsys = Convert-ToMsysPath (Join-Path $SdkDirectory "native\sysroot")
  $archIncludeMsys = "$sdkSysrootMsys/usr/include/$SysrootIncludeDir"
  $usrIncludeMsys = "$sdkSysrootMsys/usr/include"
  $libcxxIncludeDir = Resolve-OhosLibcxxIncludeDirectory -SdkDirectory $SdkDirectory -MsysBashExe $MsysBashExe
  if ($libcxxIncludeDir) {
    $libcxxIncludeMsys = Convert-ToMsysPath $libcxxIncludeDir
    $ohosCxxStdFlags = "-nostdinc++ -isystem $libcxxIncludeMsys"
  } else {
    $ohosCxxStdFlags = ""
  }
  $sourceMsys = Convert-ToMsysPath $sourceDirectory
  $installMsys = Convert-ToMsysPath $installRoot
  $workRoot = Join-Path $BuildRoot "external-src\libvpx-$version-build"
  New-Item -ItemType Directory -Path $workRoot -Force | Out-Null
  $bashScriptPath = Join-Path $workRoot "build-libvpx-ohos.sh"
$bashScriptContent = @"
set -euo pipefail
export PATH="/usr/bin:${sdkLlvmBinMsys}:`$PATH"
cd "$sourceMsys"
rm -rf build-ohos
mkdir -p build-ohos
cd build-ohos
export CC="$sdkLlvmBinMsys/clang.exe"
export CXX="$sdkLlvmBinMsys/clang++.exe"
export AS="$sdkLlvmBinMsys/clang.exe"
export LD="$sdkLlvmBinMsys/ld.lld.exe"
export AR="$sdkLlvmBinMsys/llvm-ar.exe"
export RANLIB="$sdkLlvmBinMsys/llvm-ranlib.exe"
export NM="$sdkLlvmBinMsys/llvm-nm.exe"
export STRIP=":"
export VPX_OHOS_CXXFLAGS="$ohosCxxStdFlags"
export CFLAGS="--target=$BindgenTarget --sysroot=$sdkSysrootMsys -I$archIncludeMsys -I$usrIncludeMsys -D__MUSL__ -fPIC -O2"
export CXXFLAGS="--target=$BindgenTarget --sysroot=$sdkSysrootMsys -I$archIncludeMsys -I$usrIncludeMsys `$VPX_OHOS_CXXFLAGS -D__MUSL__ -fPIC -O2"
export LDFLAGS="--target=$BindgenTarget --sysroot=$sdkSysrootMsys"
../configure --target=arm64-linux-gcc --prefix="$installMsys" --libdir="$installMsys/lib" --extra-cxxflags="`$VPX_OHOS_CXXFLAGS" --enable-static --disable-shared --disable-examples --disable-tools --disable-docs --disable-unit-tests --disable-install-bins --disable-install-srcs --disable-dependency-tracking --disable-runtime-cpu-detect --enable-vp8 --enable-vp9 --enable-vp9-highbitdepth
make -j$jobs libvpx.a
mkdir -p "$installMsys/include/vpx" "$installMsys/lib/pkgconfig"
cp -f libvpx.a "$installMsys/lib/libvpx.a"
for header in vp8.h vp8cx.h vp8dx.h vpx_codec.h vpx_decoder.h vpx_encoder.h vpx_ext_ratectrl.h vpx_frame_buffer.h vpx_image.h vpx_integer.h vpx_tpl.h; do
  cp -f "../vpx/`$header" "$installMsys/include/vpx/`$header"
done
cat > "$installMsys/lib/pkgconfig/vpx.pc" <<'PC'
# pkg-config file from libvpx v$version
prefix=$installMsys
exec_prefix=`${prefix}
libdir=`${prefix}/lib
includedir=`${prefix}/include

Name: vpx
Description: WebM Project VPx codec implementation
Version: $version
Requires:
Conflicts:
Libs: -L`${libdir} -lvpx -lm
Libs.private: -lm -lpthread
Cflags: -I`${includedir}
PC
"@
  Set-Content -Path $bashScriptPath -Value $bashScriptContent -Encoding ascii

  Write-Log "Building libvpx $version for OHOS..."
  Write-Log "  Source Dir: $sourceDirectory"
  Write-Log "  Install Root: $installRoot"
  Write-Log "  libc++ Include: $(if ($libcxxIncludeDir) { $libcxxIncludeDir } else { 'not used; only libvpx.a target is built' })"
  Write-Log "  libvpx extra CXXFLAGS: $ohosCxxStdFlags"
  $libvpxLogFile = Join-Path $workRoot "libvpx-build.log"
  & cmd.exe /d /c "`"$MsysBashExe`" `"$bashScriptPath`" > `"$libvpxLogFile`" 2>&1"
  $libvpxExitCode = $LASTEXITCODE

  if (Test-Path $libvpxLogFile) {
    Get-Content $libvpxLogFile | ForEach-Object {
      Write-Log $_
    }
  }

  if ($libvpxExitCode -ne 0) {
    throw "Failed to build libvpx for OHOS (exit code: $libvpxExitCode)."
  }

  $missingHeaders = $requiredHeaders | Where-Object { -not (Test-Path $_) }
  if (($missingHeaders.Count -gt 0) -or (-not (Test-Path $finalLib))) {
    throw "libvpx build completed, but required files were not produced: $($missingHeaders -join ', ') / $finalLib"
  }

  Write-Log "libvpx built successfully at: $finalLib"
}

function Ensure-LibyuvStaticLibrary {
  param(
    [string]$BuildRoot,
    [string]$SdkDirectory,
    [string]$MsysBashExe,
    [string]$VcpkgInstalledRoot,
    [string]$BindgenTarget
  )

  $triplet = "arm64-linux"
  $installRoot = Join-Path $VcpkgInstalledRoot $triplet
  $includeDir = Join-Path $installRoot "include"
  $libDir = Join-Path $installRoot "lib"
  $header = Join-Path $includeDir "libyuv\convert_argb.h"
  $finalLib = Join-Path $libDir "libyuv.a"
  if ((Test-Path $header) -and (Test-Path $finalLib)) {
    Write-Log "libyuv already available at: $finalLib"
    return
  }

  $revision = "0faf8dd0e004520a61a603a4d2996d5ecc80dc3f"
  $sourceDirectory = Join-Path $BuildRoot "external-src\libyuv-$revision"
  $archivePath = Join-Path $BuildRoot "downloads\libyuv-$revision.tar.gz"
  Ensure-ArchiveSource `
    -Name "libyuv $revision" `
    -Url "https://github.com/lemenkov/libyuv/archive/$revision.tar.gz" `
    -ArchivePath $archivePath `
    -SourceDirectory $sourceDirectory `
    -RequiredRelativePath "CMakeLists.txt" `
    -BuildRoot $BuildRoot

  $cmakeExe = (Get-Command cmake -ErrorAction SilentlyContinue).Source
  $ninjaExe = (Get-Command ninja -ErrorAction SilentlyContinue).Source
  if (-not $cmakeExe) {
    throw "cmake.exe was not found. Install CMake before building libyuv."
  }
  if (-not $ninjaExe) {
    throw "ninja.exe was not found. Install Ninja before building libyuv."
  }

  $jobs = Get-BuildJobCount
  $buildDir = Join-Path $BuildRoot "external-src\libyuv-$revision-build"
  Remove-DirectoryInsideRoot -Path $buildDir -RootPath $BuildRoot -Description "libyuv build directory"
  New-Item -ItemType Directory -Path $buildDir, $includeDir, $libDir -Force | Out-Null

  $sdkLlvmBin = Join-Path $SdkDirectory "native\llvm\bin"
  $sdkSysroot = Join-Path $SdkDirectory "native\sysroot"
  $clang = Convert-ToForwardSlashPath (Join-Path $sdkLlvmBin "clang.exe")
  $clangxx = Convert-ToForwardSlashPath (Join-Path $sdkLlvmBin "clang++.exe")
  $llvmAr = Convert-ToForwardSlashPath (Join-Path $sdkLlvmBin "llvm-ar.exe")
  $llvmRanlib = Convert-ToForwardSlashPath (Join-Path $sdkLlvmBin "llvm-ranlib.exe")
  $sdkSysrootForward = Convert-ToForwardSlashPath $sdkSysroot
  $installRootForward = Convert-ToForwardSlashPath $installRoot
  $cFlags = "--target=$BindgenTarget --sysroot=$sdkSysrootForward -D__MUSL__ -fPIC -O2"
  $libcxxIncludeDir = Resolve-OhosLibcxxIncludeDirectory -SdkDirectory $SdkDirectory -MsysBashExe $MsysBashExe
  if ($libcxxIncludeDir) {
    $libcxxIncludeForward = Convert-ToForwardSlashPath $libcxxIncludeDir
    $cxxFlags = "$cFlags -nostdinc++ -isystem $libcxxIncludeForward"
  } else {
    $cxxFlags = $cFlags
  }

  Write-Log "Building libyuv $revision for OHOS..."
  Write-Log "  Source Dir: $sourceDirectory"
  Write-Log "  Build Dir: $buildDir"
  Write-Log "  Install Root: $installRoot"
  Write-Log "  libc++ Include: $(if ($libcxxIncludeDir) { $libcxxIncludeDir } else { 'not used' })"
  & $cmakeExe `
    -S $sourceDirectory `
    -B $buildDir `
    -G Ninja `
    "-DCMAKE_MAKE_PROGRAM=$ninjaExe" `
    "-DCMAKE_SYSTEM_NAME=Linux" `
    "-DCMAKE_SYSTEM_PROCESSOR=aarch64" `
    "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY" `
    "-DCMAKE_BUILD_TYPE=Release" `
    "-DCMAKE_C_COMPILER=$clang" `
    "-DCMAKE_CXX_COMPILER=$clangxx" `
    "-DCMAKE_AR=$llvmAr" `
    "-DCMAKE_RANLIB=$llvmRanlib" `
    "-DCMAKE_C_FLAGS=$cFlags" `
    "-DCMAKE_CXX_FLAGS=$cxxFlags" `
    "-DCMAKE_EXE_LINKER_FLAGS=--target=$BindgenTarget --sysroot=$sdkSysrootForward" `
    "-DCMAKE_POSITION_INDEPENDENT_CODE=ON" `
    "-DCMAKE_DISABLE_FIND_PACKAGE_JPEG=ON" `
    "-DTEST=OFF" `
    "-DCMAKE_INSTALL_PREFIX=$installRootForward"
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to configure libyuv for OHOS."
  }

  & $cmakeExe --build $buildDir --config Release --target yuv --parallel $jobs
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to build libyuv for OHOS."
  }

  $builtLib = Join-Path $buildDir "libyuv.a"
  if (-not (Test-Path $builtLib)) {
    throw "libyuv build completed, but $builtLib was not produced."
  }

  Copy-Item -LiteralPath $builtLib -Destination $finalLib -Force
  $sourceInclude = Join-Path $sourceDirectory "include\libyuv"
  $destInclude = Join-Path $includeDir "libyuv"
  New-Item -ItemType Directory -Path $destInclude -Force | Out-Null
  Copy-Item -Path (Join-Path $sourceInclude "*") -Destination $destInclude -Recurse -Force

  if (-not ((Test-Path $header) -and (Test-Path $finalLib))) {
    throw "libyuv build completed, but required files were not produced: $header / $finalLib"
  }

  Write-Log "libyuv built successfully at: $finalLib"
}

function Ensure-VideoCodecStaticLibraries {
  param(
    [string]$BuildRoot,
    [string]$SdkDirectory,
    [string]$MsysBashExe,
    [string]$VcpkgInstalledRoot,
    [string]$BindgenTarget,
    [string]$SysrootIncludeDir
  )

  Ensure-LibvpxStaticLibrary `
    -BuildRoot $BuildRoot `
    -SdkDirectory $SdkDirectory `
    -MsysBashExe $MsysBashExe `
    -VcpkgInstalledRoot $VcpkgInstalledRoot `
    -BindgenTarget $BindgenTarget `
    -SysrootIncludeDir $SysrootIncludeDir

  Ensure-LibyuvStaticLibrary `
    -BuildRoot $BuildRoot `
    -SdkDirectory $SdkDirectory `
    -MsysBashExe $MsysBashExe `
    -VcpkgInstalledRoot $VcpkgInstalledRoot `
    -BindgenTarget $BindgenTarget
}

function Ensure-LibsodiumStaticLibrary {
  param(
    [string]$TargetTriple,
    [string]$BuildRoot,
    [string]$SdkDirectory,
    [string]$MsysBashExe,
    [string]$PSScriptRoot,
    [string]$BindgenTarget,
    [string]$SysrootIncludeDir,
    [string]$ConfigureHost
  )

  $installDir = Join-Path $BuildRoot "build\libsodium\$TargetTriple"
  $libDir = Join-Path $installDir "lib"
  $finalLib = Join-Path $libDir "liblibsodium.a"
  if (Test-Path $finalLib) {
    Write-Log "libsodium already built at: $finalLib"
    Stage-LibsodiumHostImportLibrary -LibDirectory $libDir
    return $libDir
  }

  $crateDirectory = Resolve-LibsodiumCrateDirectory
  $workRoot = Join-Path $BuildRoot "external-src\libsodium-$TargetTriple"
  $sourceRoot = Join-Path $workRoot "source"
  $sourceDirectory = Join-Path $sourceRoot "libsodium"
  if (Test-Path $sourceRoot) {
    Remove-Item -LiteralPath $sourceRoot -Recurse -Force
  }
  if (Test-Path $installDir) {
    Remove-Item -LiteralPath $installDir -Recurse -Force
  }
  New-Item -ItemType Directory -Path $sourceRoot, $installDir -Force | Out-Null
  Copy-Item -Path (Join-Path $crateDirectory "libsodium") -Destination $sourceDirectory -Recurse -Force

  $jobs = Get-BuildJobCount

  $linkerMsys = Convert-ToMsysPath (Join-Path $PSScriptRoot "$TargetTriple-clang.cmd")
  $sdkLlvmBin = Join-Path $SdkDirectory "native\llvm\bin"
  $sdkLlvmBinMsys = Convert-ToMsysPath $sdkLlvmBin
  $sdkSysrootMsys = Convert-ToMsysPath (Join-Path $SdkDirectory "native\sysroot")
  $archIncludeMsys = "$sdkSysrootMsys/usr/include/$SysrootIncludeDir"
  $usrIncludeMsys = "$sdkSysrootMsys/usr/include"
  $installMsys = Convert-ToMsysPath $installDir
  $sourceMsys = Convert-ToMsysPath $sourceDirectory
  $bashScriptPath = Join-Path $workRoot "build-libsodium.sh"
  $bashScriptContent = @"
set -euo pipefail
cd "$sourceMsys"
export PATH="/usr/bin:${sdkLlvmBinMsys}:`$PATH"
export RUSTDESK_HARMONY_HOST_SDK="$SdkDirectory"
export OHOS_SDK_HOME="$SdkDirectory"
export OHOS_NDK_HOME="$SdkDirectory"
export CC="$linkerMsys"
export LD="$sdkLlvmBinMsys/ld.lld.exe"
export AR="$sdkLlvmBinMsys/llvm-ar.exe"
export RANLIB="$sdkLlvmBinMsys/llvm-ranlib.exe"
export NM="$sdkLlvmBinMsys/llvm-nm.exe"
export STRIP=":"
export LDCONFIG=":"
export CFLAGS="--target=$BindgenTarget --sysroot=$sdkSysrootMsys -I$archIncludeMsys -I$usrIncludeMsys -D__MUSL__"
./configure --host=$ConfigureHost --prefix="$installMsys" --libdir="$installMsys/lib" --enable-shared=no
make -j$jobs all
make install
cp -f "$installMsys/lib/libsodium.a" "$installMsys/lib/liblibsodium.a"
"@
  Set-Content -Path $bashScriptPath -Value $bashScriptContent -Encoding ascii

  Write-Log "Building external libsodium for $TargetTriple..."
  Write-Log "  Work Root: $workRoot"
  Write-Log "  Source Dir: $sourceDirectory"
  Write-Log "  Install Dir: $installDir"
  Write-Log "  Build Jobs: $jobs"
 $libsodiumLogFile = Join-Path $workRoot "libsodium-build.log"

  & cmd.exe /d /c "`"$MsysBashExe`" `"$bashScriptPath`" > `"$libsodiumLogFile`" 2>&1"
  $libsodiumExitCode = $LASTEXITCODE

  if (Test-Path $libsodiumLogFile) {
    Get-Content $libsodiumLogFile | ForEach-Object {
      Write-Log $_
    }
  }

  if ($libsodiumExitCode -ne 0) {
    Write-Log "ERROR: Failed to build libsodium for $TargetTriple (exit code: $libsodiumExitCode)"
    throw "Failed to build libsodium for $TargetTriple."
  }

  if (-not (Test-Path $finalLib)) {
    Write-Log "ERROR: libsodium build completed, but $finalLib was not produced."
    throw "libsodium build completed, but $finalLib was not produced."
  }

  Write-Log "libsodium built successfully at: $finalLib"
  Stage-LibsodiumHostImportLibrary -LibDirectory $libDir
  return $libDir
}

if (Get-Command cargo -ErrorAction SilentlyContinue) {
  $cargoExe = (Get-Command cargo).Source
} elseif (Test-Path "$env:USERPROFILE\.cargo\bin\cargo.exe") {
  $cargoExe = "$env:USERPROFILE\.cargo\bin\cargo.exe"
}

if (Get-Command rustup -ErrorAction SilentlyContinue) {
  $rustupExe = (Get-Command rustup).Source
} elseif (Test-Path "$env:USERPROFILE\.cargo\bin\rustup.exe") {
  $rustupExe = "$env:USERPROFILE\.cargo\bin\rustup.exe"
}

$vcvarsCandidates = @()
if ($env:VISUAL_STUDIO_VCVARS64 -and (Test-Path $env:VISUAL_STUDIO_VCVARS64)) {
  $vcvarsCandidates += $env:VISUAL_STUDIO_VCVARS64
}
$vcvarsCandidates += @(
  "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
  "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
  "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat",
  "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat",
  "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
)
foreach ($candidate in $vcvarsCandidates) {
  if ($candidate -and (Test-Path $candidate)) {
    $vcvarsScript = $candidate
    break
  }
}

$pathBash = Get-Command "bash.exe" -ErrorAction SilentlyContinue
$setupBash = Resolve-MsysSetupToolPath -MsysPath "/usr/bin/bash.exe"
$msysBashExe = Resolve-MsysTool -Candidates @(
  $setupBash,
  $(if ($pathBash) { $pathBash.Source } else { $null }),
  "C:\msys64\usr\bin\bash.exe",
  "$env:USERPROFILE\scoop\apps\msys2\current\usr\bin\bash.exe"
) -Description "MSYS2 bash.exe"
$msysBinDir = Split-Path -Parent $msysBashExe
$pathPerl = Get-Command "perl.exe" -ErrorAction SilentlyContinue
$setupPerl = Resolve-MsysSetupToolPath -MsysPath "/usr/bin/perl.exe"
$msysPerlExe = Resolve-MsysTool -Candidates @(
  $setupPerl,
  (Join-Path $msysBinDir "perl.exe"),
  $(if ($pathPerl) { $pathPerl.Source } else { $null }),
  "C:\msys64\usr\bin\perl.exe",
  "$env:USERPROFILE\scoop\apps\msys2\current\usr\bin\perl.exe"
) -Description "MSYS2 perl.exe"

if (-not $cargoExe) {
  throw "cargo.exe was not found. Install Rust or add cargo to PATH before building the Harmony native bridge."
}

if (-not (Test-Path $linkerScript)) {
  throw "OpenHarmony linker wrapper was not found: $linkerScript"
}

if (-not (Test-Path $llvmArScript)) {
  throw "OpenHarmony llvm-ar wrapper was not found: $llvmArScript"
}

if (-not (Test-Path $ohosEnvScript)) {
  throw "OpenHarmony SDK env wrapper was not found: $ohosEnvScript"
}

if (-not (Test-Path $vcpkgInstalledRoot)) {
  throw "vcpkg installed root was not found: $vcpkgInstalledRoot"
}

$resolvedHostSdkDir = Resolve-HostSdkDirectory -BuildRoot $buildRoot -LocalPropertiesFile $localProperties
if (-not $resolvedHostSdkDir) {
  throw "OpenHarmony host SDK was not found. Install the DevEco SDK or set OHOS_SDK_HOME/OHOS_NDK_HOME."
}
$hostSdkDir = Ensure-NoSpaceSdkMirror -SdkDirectory $resolvedHostSdkDir -MirrorDirectory $hostSdkMirrorDir
Write-Log "Host SDK Dir: $hostSdkDir"
$env:RUSTDESK_HARMONY_HOST_SDK = $hostSdkDir
$env:OHOS_SDK_HOME = $hostSdkDir
$env:OHOS_NDK_HOME = $hostSdkDir

$sdkLlvmBin = Join-Path $hostSdkDir "native\llvm\bin"
$sdkLdExe = Join-Path $sdkLlvmBin "ld.lld.exe"
$sdkNmExe = Join-Path $sdkLlvmBin "llvm-nm.exe"
$sdkRanlibExe = Join-Path $sdkLlvmBin "llvm-ranlib.exe"
if (-not (Test-Path $sdkLdExe)) {
  throw "OpenHarmony linker executable was not found under $sdkLlvmBin."
}
if (-not (Test-Path $sdkNmExe)) {
  throw "OpenHarmony llvm-nm executable was not found under $sdkLlvmBin."
}
if (-not (Test-Path $sdkRanlibExe)) {
  throw "OpenHarmony llvm-ranlib executable was not found under $sdkLlvmBin."
}

$libsodiumLibDir = Ensure-LibsodiumStaticLibrary `
  -TargetTriple $TargetTriple `
  -BuildRoot $buildRoot `
  -SdkDirectory $hostSdkDir `
  -MsysBashExe $msysBashExe `
  -PSScriptRoot $PSScriptRoot `
  -BindgenTarget $bindgenTarget `
  -SysrootIncludeDir $sysrootIncludeDir `
  -ConfigureHost $configureHost

Ensure-VideoCodecStaticLibraries `
  -BuildRoot $buildRoot `
  -SdkDirectory $hostSdkDir `
  -MsysBashExe $msysBashExe `
  -VcpkgInstalledRoot $vcpkgInstalledRoot `
  -BindgenTarget $bindgenTarget `
  -SysrootIncludeDir $sysrootIncludeDir

New-Item -ItemType Directory -Path $cargoTargetDir -Force | Out-Null

$staleRoots = @(
  (Join-Path $cargoTargetDir "release\build"),
  (Join-Path $cargoTargetDir "release\.fingerprint"),
  (Join-Path $cargoTargetDir "$TargetTriple\$Profile\build"),
  (Join-Path $cargoTargetDir "$TargetTriple\$Profile\.fingerprint")
)
Write-Log "Cleaning stale build artifacts..."
foreach ($staleRoot in $staleRoots) {
  if (-not (Test-Path $staleRoot)) {
    continue
  }
  Get-ChildItem -Path $staleRoot -Directory -ErrorAction SilentlyContinue | Where-Object {
    $_.Name -like "openssl-sys-*" -or $_.Name -like "openssl-*"
  } | ForEach-Object {
    Write-Log "  Removing: $($_.FullName)"
    Remove-Item -LiteralPath $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
  }
}

$linkerForward = Convert-ToForwardSlashPath $linkerScript
$cxxForward = Convert-ToForwardSlashPath $cxxScript
$llvmArForward = Convert-ToForwardSlashPath $llvmArScript
$sdkLdForward = Convert-ToForwardSlashPath $sdkLdExe
$sdkNmForward = Convert-ToForwardSlashPath $sdkNmExe
$sdkRanlibForward = Convert-ToForwardSlashPath $sdkRanlibExe
$msysPerlForward = Convert-ToForwardSlashPath $msysPerlExe
$sdkClangForward = Convert-ToForwardSlashPath (Join-Path $sdkLlvmBin "clang.exe")
$sdkClangxxForward = Convert-ToForwardSlashPath (Join-Path $sdkLlvmBin "clang++.exe")
$sdkLlvmArForward = Convert-ToForwardSlashPath (Join-Path $sdkLlvmBin "llvm-ar.exe")
$sdkSysrootForward = Convert-ToForwardSlashPath (Join-Path $hostSdkDir "native\sysroot")
$bindgenArchIncludeForward = "$sdkSysrootForward/usr/include/$sysrootIncludeDir"
$bindgenUsrIncludeForward = "$sdkSysrootForward/usr/include"
$bindgenClangArgs = "--target=$bindgenTarget --sysroot=$sdkSysrootForward -isystem $bindgenArchIncludeForward -isystem $bindgenUsrIncludeForward -D__MUSL__"
$targetCompileFlags = "--target=$bindgenTarget --sysroot=$sdkSysrootForward -D__MUSL__ -fPIC"
$libclangPathForward = Convert-ToForwardSlashPath $sdkLlvmBin

$cmdLines = @(
  "@echo off",
  "setlocal enabledelayedexpansion",
  $(if ($vcvarsScript) { "call `"$vcvarsScript`" >nul || exit /b 1" } else { "rem vcvars64.bat not found; using current environment" }),
  "set `"RUSTDESK_HARMONY_HOST_SDK=$hostSdkDir`"",
  "set `"OHOS_SDK_HOME=$hostSdkDir`"",
  "call `"$ohosEnvScript`" || exit /b 1",
  "set `"PATH=%LLVM_BIN%;$env:USERPROFILE\.cargo\bin;%PATH%;$msysBinDir`"",
  "set `"LIBCLANG_PATH=$libclangPathForward`"",
  $(if ($env:RUST_TOOLCHAIN_VERSION) { "set `"RUSTUP_TOOLCHAIN=$($env:RUST_TOOLCHAIN_VERSION)`"" } else { "set `"RUSTUP_TOOLCHAIN=stable`"" }),
  "set `"CARGO_TARGET_DIR=$cargoTargetDir`"",
  "set `"VCPKG_ROOT=$vcpkgRoot`"",
  "set `"VCPKG_INSTALLED_ROOT=$vcpkgInstalledRoot`"",
  "set `"SODIUM_LIB_DIR=$libsodiumLibDir`"",
  "set `"PERL=$msysPerlForward`"",
  "set `"OPENSSL_SRC_PERL=$msysPerlForward`"",
  "set `"LD=$sdkLdForward`"",
  "set `"NM=$sdkNmForward`"",
  "set `"RANLIB=$sdkRanlibForward`"",
  "set `"CARGO_TARGET_${cargoTargetKey}_LINKER=$linkerScript`"",
  "set `"CARGO_TARGET_${cargoTargetKey}_AR=$llvmArScript`"",
  "set `"CC_${cargoTargetKey}=$sdkClangForward`"",
  "set `"CXX_${cargoTargetKey}=$sdkClangxxForward`"",
  "set `"AR_${cargoTargetKey}=$sdkLlvmArForward`"",
  "set `"CC_${TargetTriple}=$sdkClangForward`"",
  "set `"CXX_${TargetTriple}=$sdkClangxxForward`"",
  "set `"AR_${TargetTriple}=$sdkLlvmArForward`"",
  "set `"CC_${targetEnvKey}=$sdkClangForward`"",
  "set `"CXX_${targetEnvKey}=$sdkClangxxForward`"",
  "set `"AR_${targetEnvKey}=$sdkLlvmArForward`"",
  "set `"CFLAGS_${TargetTriple}=$targetCompileFlags`"",
  "set `"CXXFLAGS_${TargetTriple}=$targetCompileFlags`"",
  "set `"CFLAGS_${targetEnvKey}=$targetCompileFlags`"",
  "set `"CXXFLAGS_${targetEnvKey}=$targetCompileFlags`"",
  "set `"LD_${targetEnvKey}=$sdkLdForward`"",
  "set `"NM_${targetEnvKey}=$sdkNmForward`"",
  "set `"RANLIB_${targetEnvKey}=$sdkRanlibForward`"",
  "set `"BINDGEN_EXTRA_CLANG_ARGS=$bindgenClangArgs`"",
  "set `"BINDGEN_EXTRA_CLANG_ARGS_${TargetTriple}=$bindgenClangArgs`"",
  "set `"BINDGEN_EXTRA_CLANG_ARGS_${targetEnvKey}=$bindgenClangArgs`"",
  $(if ($rustupExe) { "call `"$rustupExe`" target add $TargetTriple || exit /b 1" } else { "rem rustup.exe not found; assuming target $TargetTriple is already installed" }),
  "echo === Cargo Build Environment ===",
  "echo Target Triple: $TargetTriple",
  "echo Cargo: `"$cargoExe`"",
  "echo LD: !LD!",
  "echo CC: !CC_${targetEnvKey}!",
  "echo CXX: !CXX_${targetEnvKey}!",
  "echo AR: !AR_${targetEnvKey}!",
  "echo ===",
  "echo === Cargo Build Environment ===",
  "echo RUSTUP_TOOLCHAIN=%RUSTUP_TOOLCHAIN%",
  "`"$cargoExe`" --version",
  "rustc --version --verbose",
  "cd /d `"$nativeCoreDir`"",
  "echo Working directory: %cd%",
  "echo === Starting Cargo Build ===",
  "`"$cargoExe`" build --profile $Profile --target $TargetTriple -vv",
  "set BUILD_EXIT_CODE=!ERRORLEVEL!",
  "echo === Cargo Build Finished: !BUILD_EXIT_CODE! ===",
  "exit /b !BUILD_EXIT_CODE!"
)

$cmdScript = [string]::Join("`r`n", $cmdLines)
$cmdScriptPath = Join-Path $buildRoot "build-native-bridge-$TargetTriple.cmd"
Write-Log "Build script path: $cmdScriptPath"
Set-Content -Path $cmdScriptPath -Value $cmdScript -Encoding ascii

# 保存环境信息用于调试
Write-Log "=== Saving Build Environment Information ==="
@"
=== Build Environment Snapshot ===
Timestamp: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Working Directory: $(Get-Location)
PowerShell Version: $($PSVersionTable.PSVersion)
OS Version: $([System.Environment]::OSVersion)

=== Key Paths ===
cargo.exe: $cargoExe
rustup.exe: $rustupExe
MSYS2 bash: $msysBashExe
MSYS2 perl: $msysPerlExe
Host SDK Dir: $hostSdkDir
Native Core Dir: $nativeCoreDir
Cargo Target Dir: $cargoTargetDir
Build Root: $buildRoot

=== Environment Variables ===
"@ | Out-File -FilePath $envLogFile -Encoding UTF8
Get-ChildItem env: | Where-Object { $_.Name -like "*CARGO*" -or $_.Name -like "*RUST*" -or $_.Name -like "*OHOS*" -or $_.Name -like "*LLVM*" } | ForEach-Object {
  "$($_.Name)=$($_.Value)" | Out-File -FilePath $envLogFile -Encoding UTF8 -Append
}

Write-Log "Environment log saved to: $envLogFile"
Write-Log ""
Write-Log "=== Starting Cargo Build ==="
Write-Log "Running: & cmd.exe /d /c $cmdScriptPath"
Write-Log "Current working directory: $(Get-Location)"

try {
  # 通过 cmd.exe 自己重定向日志，避免 PowerShell 把 stderr 当 NativeCommandError
  $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()

  & cmd.exe /d /c "`"$cmdScriptPath`" > `"$cargoLogFile`" 2>&1"
  $cargoExitCode = $LASTEXITCODE

  if (Test-Path $cargoLogFile) {
    Get-Content $cargoLogFile | ForEach-Object {
      Write-Host $_
    }
  }

  $stopwatch.Stop()

  Write-Log "Cargo build took $($stopwatch.Elapsed.TotalSeconds) seconds"
  Write-Log "Cargo build exit code: $cargoExitCode"
  
  # 检查 cargo 进程是否崩溃
  if ($cargoExitCode -eq -1073741819) {
    Write-Log "ERROR: Access violation (0xC0000005) - likely memory corruption or tool chain issue"
    Write-Log "Possible causes:"
    Write-Log "  1. Insufficient memory or disk space"
    Write-Log "  2. CARGO_BUILD_JOBS too high - try reducing to 1"
    Write-Log "  3. Parallel compilation conflict"
    Write-Log "  4. MSYS2/LLVM toolchain incompatibility"
  }
  
  # 读取并显示 cargo 日志的最后部分
  if (Test-Path $cargoLogFile) {
    Write-Log ""
    Write-Log "=== Cargo build output (last 100 lines) ==="
    $cargoLog = Get-Content -Path $cargoLogFile -Tail 100 -ErrorAction SilentlyContinue
    if ($cargoLog) {
      $cargoLog | ForEach-Object { Write-Log $_ }
    }
  }
  
  if ($cargoExitCode -ne 0) {
    Write-Log "ERROR: cargo build failed with exit code $cargoExitCode"
    Write-Log "Full cargo build log: $cargoLogFile"
    Write-Log "Environment log: $envLogFile"
    Write-Log "Script log file: $logFile"
    Write-Log ""
    Write-Log "Debugging tips:"
    Write-Log "  1. Check memory usage during build"
    Write-Log "  2. Try setting CARGO_BUILD_JOBS=1 in environment"
    Write-Log "  3. Increase system page file size"
    Write-Log "  4. Clear cargo incremental compilation cache"
    throw "cargo build failed with exit code $cargoExitCode."
  }
} finally {
  Write-Log "Debug cmd script kept at: $cmdScriptPath"
  Write-Log "Build script cleaned up"
}

Write-Log "=== Locating built artifact ==="
$artifactProfileDir = switch ($Profile) {
  "dev" { "debug" }
  default { $Profile }
}
$artifactDir = Join-Path $cargoTargetDir "$TargetTriple\$artifactProfileDir"
Write-Log "Artifact directory: $artifactDir"

$staticLib = Join-Path $artifactDir "rustdesk_harmony_bridge.a"
$prefixedStaticLib = Join-Path $artifactDir "librustdesk_harmony_bridge.a"
$depsStaticLib = Join-Path $artifactDir "deps\librustdesk_harmony_bridge.a"

Write-Log "Checking library locations:"
Write-Log "  $prefixedStaticLib exists: $(Test-Path $prefixedStaticLib)"
Write-Log "  $staticLib exists: $(Test-Path $staticLib)"
Write-Log "  $depsStaticLib exists: $(Test-Path $depsStaticLib)"

# List directory contents for debugging
if (Test-Path $artifactDir) {
  Write-Log "Contents of $artifactDir :"
  Get-ChildItem -Path $artifactDir -File | ForEach-Object {
    Write-Log "  - $($_.Name) ($($_.Length) bytes)"
  }
}

if (Test-Path $prefixedStaticLib) {
  $sourceLib = $prefixedStaticLib
} elseif (Test-Path $staticLib) {
  $sourceLib = $staticLib
} elseif (Test-Path $depsStaticLib) {
  $sourceLib = $depsStaticLib
} else {
  Write-Log "ERROR: Native bridge build succeeded, but no static library was found in $artifactDir."
  Write-Log "Full cargo build log: $cargoLogFile"
  Write-Log "Environment log: $envLogFile"
  Write-Log "Script log file: $logFile"
  throw "Native bridge build succeeded, but no static library was found in $artifactDir."
}

Write-Log "Using source library: $sourceLib"

New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
Copy-Item -LiteralPath $sourceLib -Destination (Join-Path $outputDir "librustdesk_harmony_bridge.a") -Force
Write-Log "Copied to: $outputDir\librustdesk_harmony_bridge.a"

$appStaticLib = Join-Path $projectRoot "entry\src\main\libs\arm64\librustdesk_core.a"
New-Item -ItemType Directory -Path (Split-Path -Parent $appStaticLib) -Force | Out-Null
Copy-Item -LiteralPath $sourceLib -Destination $appStaticLib -Force
Write-Log "Copied to: $appStaticLib"

Write-Log "=== Build Native Bridge Completed Successfully ==="
Write-Log "Native bridge artifact copied to $outputDir\librustdesk_harmony_bridge.a"
Write-Log "App native core staticlib updated at $appStaticLib"
Write-Log "Script log: $logFile"
Write-Log "Cargo build log: $cargoLogFile"
Write-Log "Environment log: $envLogFile"

Write-Host "Native bridge artifact copied to $outputDir\librustdesk_harmony_bridge.a"
Write-Host "App native core staticlib updated at $appStaticLib"
Write-Host "Script log: $logFile"
Write-Host "Cargo build log: $cargoLogFile"
Write-Host "Environment log: $envLogFile"
