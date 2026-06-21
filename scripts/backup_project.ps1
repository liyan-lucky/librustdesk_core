param(
  [int]$Keep = 2
)

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$workspaceRoot = [System.IO.Path]::GetFullPath((Join-Path $projectRoot ".."))
$tempRoot = [System.IO.Path]::GetFullPath((Join-Path $workspaceRoot "99_Temp"))
$backupRoot = Join-Path $tempRoot "rustdesk_core_backups"
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$stageRoot = Join-Path $env:TEMP "rustdesk_core_backup_$timestamp"
$zipPath = Join-Path $backupRoot "rustdesk_core_$timestamp.zip"
$hashPath = "$zipPath.sha256"

New-Item -ItemType Directory -Force -Path $backupRoot | Out-Null

function Remove-DirectorySafe {
  param([Parameter(Mandatory = $true)][string]$Path)

  $fullPath = [System.IO.Path]::GetFullPath($Path)
  $tempPath = [System.IO.Path]::GetFullPath($env:TEMP)
  if (-not $fullPath.StartsWith($tempPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove path outside TEMP: $fullPath"
  }

  if (-not (Test-Path -LiteralPath $fullPath)) {
    return
  }

  try {
    Get-ChildItem -LiteralPath $fullPath -Recurse -Force -ErrorAction SilentlyContinue |
      ForEach-Object { $_.Attributes = [System.IO.FileAttributes]::Normal }
  } catch {}

  $longPath = "\\?\$fullPath"
  [System.IO.Directory]::Delete($longPath, $true)
}

if (Test-Path -LiteralPath $stageRoot) {
  Remove-DirectorySafe -Path $stageRoot
}

$excludeDirs = @(
  (Join-Path $projectRoot ".git"),
  (Join-Path $projectRoot ".codeartsdoer"),
  (Join-Path $projectRoot "entry\build"),
  (Join-Path $projectRoot "native_rust_core\target"),
  (Join-Path $projectRoot "rustdesk-master\target")
)

try {
  $robocopyArgs = @($projectRoot, $stageRoot, "/E", "/XJ", "/R:2", "/W:1", "/XD") + $excludeDirs + @(
    "/XF",
    "*.log",
    "*.tmp",
    "*.bak",
    "*.a",
    "*.so"
  )
  & robocopy @robocopyArgs | Out-Host
  $robocopyExit = $LASTEXITCODE
  if ($robocopyExit -gt 7) {
    throw "robocopy failed with exit code $robocopyExit"
  }

  if (Test-Path -LiteralPath $zipPath) {
    Remove-Item -LiteralPath $zipPath -Force
  }
  Compress-Archive -Path (Join-Path $stageRoot "*") -DestinationPath $zipPath -CompressionLevel Optimal
  $hash = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash
  [System.IO.File]::WriteAllText($hashPath, "$hash  $([System.IO.Path]::GetFileName($zipPath))`r`n")
} finally {
  if (Test-Path -LiteralPath $stageRoot) {
    Remove-DirectorySafe -Path $stageRoot
  }
}

Get-ChildItem -LiteralPath $backupRoot -Filter "rustdesk_core_*.zip" |
  Sort-Object LastWriteTime -Descending |
  Select-Object -Skip $Keep |
  ForEach-Object {
    Remove-Item -LiteralPath $_.FullName -Force
    $oldHashPath = "$($_.FullName).sha256"
    if (Test-Path -LiteralPath $oldHashPath) {
      Remove-Item -LiteralPath $oldHashPath -Force
    }
  }

Write-Host "Backup written to $zipPath"
Write-Host "SHA256 written to $hashPath"
