#Requires -Version 5.1
<#
.SYNOPSIS
    一鍵檢查工具鏈、執行測試並建置 WinNUT Rust Client（Windows 安裝檔）。

.DESCRIPTION
    步驟：
      1. 檢查 Rust (rustup/cargo)、Bun、MSVC C++ Build Tools、WebView2
      2. bun install（依 bun.lock，不升級套件）
      3. 翻譯檢查 + TypeScript 型別檢查
      4. cargo test（含 NUT 協定解析與模擬伺服器測試）
      5. bun tauri build → 產生 .msi 與 .exe 安裝檔
    完整輸出寫入專案根目錄的 checks.log。

.PARAMETER InstallMissing
    缺少的工具用 winget 自動安裝（Rust、Bun、VS 2022 Build Tools）。

.PARAMETER Dev
    不打包，改用 `bun tauri dev` 直接啟動開發版（含即時重新載入）。

.PARAMETER SkipTests
    略過 cargo test（只建置）。

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
    powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -InstallMissing
    powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -Dev
#>
param(
    [switch]$InstallMissing,
    [switch]$Dev,
    [switch]$SkipTests
)

$ErrorActionPreference = 'Continue'
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location -LiteralPath $Root
$Log = Join-Path $Root 'checks.log'
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::WriteAllText($Log, "WinNUT Rust Client build log  $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')`r`n", $utf8)
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Write-Log([string]$text) {
    [IO.File]::AppendAllText($Log, "$text`r`n", $utf8)
}

function Refresh-Path {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user = [Environment]::GetEnvironmentVariable('Path', 'User')
    $env:Path = "$machine;$user;$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.bun\bin"
}

function Test-Command([string]$name) {
    return [bool](Get-Command $name -ErrorAction SilentlyContinue)
}

# Runs a command through cmd.exe (stdout+stderr merged), shows it live and appends it to checks.log.
function Invoke-Step([string]$title, [string]$command) {
    Write-Host "==> $title" -ForegroundColor Cyan
    Write-Log "`r`n==== $title ====`r`n> $command"
    $sw = [Diagnostics.Stopwatch]::StartNew()
    cmd /d /c "$command 2>&1" | ForEach-Object {
        Write-Host "    $_"
        Write-Log $_
    }
    $code = $LASTEXITCODE
    Write-Log "EXIT $code ($([int]$sw.Elapsed.TotalSeconds)s)"
    if ($code -ne 0) {
        Write-Host "[FAIL] $title (exit $code) — 詳見 checks.log" -ForegroundColor Red
        Write-Log 'BUILD FAILED'
        exit $code
    }
    Write-Host "[ OK ] $title ($([int]$sw.Elapsed.TotalSeconds)s)" -ForegroundColor Green
}

function Install-WithWinget([string]$id, [string]$extra = '') {
    if (-not (Test-Command 'winget')) {
        Write-Host "    找不到 winget，請手動安裝 $id" -ForegroundColor Yellow
        return
    }
    Invoke-Step "winget install $id" "winget install --id $id -e --accept-source-agreements --accept-package-agreements $extra"
    Refresh-Path
}

# ---------------------------------------------------------------------------------------
# 1. Toolchain checks
# ---------------------------------------------------------------------------------------
Write-Host '==> 檢查建置工具' -ForegroundColor Cyan
Refresh-Path
$missing = @()

if (-not (Test-Command 'cargo')) {
    if ($InstallMissing) { Install-WithWinget 'Rustlang.Rustup' }
    if (-not (Test-Command 'cargo')) { $missing += 'Rust（winget install Rustlang.Rustup，或 https://rustup.rs）' }
}
if (Test-Command 'rustup') {
    # Tauri on Windows needs the MSVC toolchain, not the GNU one.
    cmd /d /c "rustup toolchain list 2>&1" | Out-Null
    $active = (cmd /d /c "rustup show active-toolchain 2>&1") -join ' '
    Write-Log "rustup active toolchain: $active"
    if ($active -notmatch 'msvc') {
        # Only for this build (does not change your global rustup default).
        Write-Host '    預設 Rust 工具鏈不是 MSVC，本次建置改用 stable-x86_64-pc-windows-msvc' -ForegroundColor Yellow
        Invoke-Step 'rustup toolchain (msvc)' 'rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal'
        $env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-msvc'
    }
}

if (-not (Test-Command 'bun')) {
    if ($InstallMissing) { Install-WithWinget 'Oven-sh.Bun' }
    if (-not (Test-Command 'bun')) { $missing += 'Bun（winget install Oven-sh.Bun，或 https://bun.sh）' }
}

# MSVC linker: located through vswhere (installed with any VS 2017+ / Build Tools).
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$hasVc = $false
if (Test-Path $vswhere) {
    $vc = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    $hasVc = [bool]$vc
}
if (-not $hasVc) {
    if ($InstallMissing) {
        Install-WithWinget 'Microsoft.VisualStudio.2022.BuildTools' '--override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
        if (Test-Path $vswhere) {
            $vc = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
            $hasVc = [bool]$vc
        }
    }
    if (-not $hasVc) { $missing += 'Visual Studio 2022 Build Tools（勾選「使用 C++ 的桌面開發」）' }
}

# WebView2 runtime ships with Windows 11; only warn if it is missing.
$wv2 = @(
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
) | Where-Object { Test-Path $_ }
if (-not $wv2) { Write-Host '    警告：未偵測到 WebView2 Runtime（Windows 11 通常內建）' -ForegroundColor Yellow }

if ($missing.Count -gt 0) {
    Write-Host "`n缺少以下建置工具：" -ForegroundColor Red
    $missing | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red; Write-Log "MISSING: $_" }
    Write-Host "`n可加上 -InstallMissing 參數自動安裝，安裝後請重新開啟 PowerShell 再執行一次。" -ForegroundColor Yellow
    exit 1
}
Invoke-Step 'cargo / rustc / bun 版本' 'cargo --version && rustc --version && bun --version'

# ---------------------------------------------------------------------------------------
# 2-4. Dependencies, checks, tests
# ---------------------------------------------------------------------------------------
Invoke-Step 'bun install' 'bun install --frozen-lockfile'
Invoke-Step '翻譯檢查 (check:i18n)' 'bun run check:i18n'
Invoke-Step 'TypeScript 型別檢查' 'bun x tsc --noEmit'
if (-not $SkipTests) {
    Invoke-Step 'cargo test' 'cd src-tauri && cargo test'
}

if ($Dev) {
    Write-Host '==> 啟動開發版（關閉視窗即結束）' -ForegroundColor Cyan
    bun tauri dev
    exit $LASTEXITCODE
}

# ---------------------------------------------------------------------------------------
# 5. Package
# ---------------------------------------------------------------------------------------
# Updater artifacts must be signed; without the private key, build the installers only.
$configArg = ''
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    $configArg = '--config scripts\tauri.local.conf.json'
    Write-Host '    未設定 TAURI_SIGNING_PRIVATE_KEY：略過自動更新簽章檔，只產生安裝檔。' -ForegroundColor DarkGray
}
Invoke-Step 'bun tauri build' "bun tauri build $configArg"

$bundle = Join-Path $Root 'src-tauri\target\release\bundle'
$artifacts = Get-ChildItem -Path $bundle -Recurse -Include *.msi, *-setup.exe -ErrorAction SilentlyContinue
Write-Host "`n建置完成，安裝檔：" -ForegroundColor Green
$artifacts | ForEach-Object { Write-Host "  $($_.FullName)" -ForegroundColor Green; Write-Log "ARTIFACT: $($_.FullName)" }
Write-Host "  執行檔：$(Join-Path $Root 'src-tauri\target\release\ups-client.exe')" -ForegroundColor Green
Write-Log 'BUILD OK'
