# clip-win 開発環境セットアップスクリプト
# =========================================
# 使い方:
#   PowerShell を「管理者権限」で開き、以下を実行:
#   .\scripts\install.ps1
#
# インストールされるもの:
#   - Visual Studio Build Tools 2022 (C++ デスクトップ開発)
#   - Rust (rustup)
#   - Node.js LTS
#   - Tauri CLI v2
# =========================================

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── ヘルパー関数 ─────────────────────────────────────────────

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host ">>> $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "  ✓ $Message" -ForegroundColor Green
}

function Write-Info {
    param([string]$Message)
    Write-Host "  • $Message" -ForegroundColor Gray
}

function Test-CommandExists {
    param([string]$Command)
    return [bool](Get-Command $Command -ErrorAction SilentlyContinue)
}

function Test-WingetInstalled {
    param([string]$PackageId)
    $result = winget list --id $PackageId 2>$null
    return $LASTEXITCODE -eq 0 -and ($result -match $PackageId)
}

# ── 管理者権限チェック ────────────────────────────────────────

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host ""
    Write-Host "[ERROR] このスクリプトは管理者権限が必要です。" -ForegroundColor Red
    Write-Host "  PowerShell を右クリック →「管理者として実行」で起動してください。" -ForegroundColor Yellow
    exit 1
}

# ── winget チェック ───────────────────────────────────────────

Write-Step "winget の確認"
if (-not (Test-CommandExists "winget")) {
    Write-Host "[ERROR] winget が見つかりません。" -ForegroundColor Red
    Write-Host "  Windows 10 1709 以降では標準搭載されています。" -ForegroundColor Yellow
    Write-Host "  https://aka.ms/getwinget からインストールしてください。" -ForegroundColor Yellow
    exit 1
}
Write-Success "winget が使用可能です"

# ── Step 1: Visual Studio Build Tools ────────────────────────

Write-Step "Step 1/4: Visual Studio Build Tools 2022 の確認"

$vsBuildToolsInstalled = $false
$vsInstances = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" `
    -products Microsoft.VisualStudio.Product.BuildTools `
    -requires Microsoft.VisualStudio.Workload.VCTools `
    -format json 2>$null | ConvertFrom-Json

if ($vsInstances -and $vsInstances.Count -gt 0) {
    $vsBuildToolsInstalled = $true
}

if ($vsBuildToolsInstalled) {
    Write-Success "Visual Studio Build Tools は既にインストール済みです"
} else {
    Write-Info "Visual Studio Build Tools をインストールします（数分かかります）..."
    winget install Microsoft.VisualStudio.2022.BuildTools `
        --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" `
        --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Visual Studio Build Tools のインストールに失敗しました。" -ForegroundColor Red
        Write-Host "  手動でインストールしてください: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor Yellow
        exit 1
    }
    Write-Success "Visual Studio Build Tools をインストールしました"
}

# ── Step 2: Rust ─────────────────────────────────────────────

Write-Step "Step 2/4: Rust (rustup) の確認"

if (Test-CommandExists "rustc") {
    $rustVersion = rustc --version
    Write-Success "Rust は既にインストール済みです: $rustVersion"
} else {
    Write-Info "Rust をインストールします..."
    winget install Rustlang.Rustup --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Rust のインストールに失敗しました。" -ForegroundColor Red
        Write-Host "  手動でインストールしてください: https://rustup.rs/" -ForegroundColor Yellow
        exit 1
    }

    # PATH を現在のセッションに反映
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH
    $cargoPath = "$env:USERPROFILE\.cargo\bin"
    if (Test-Path $cargoPath) {
        $env:PATH = "$cargoPath;$env:PATH"
    }

    Write-Success "Rust をインストールしました"
    Write-Info "※ PATH を反映するには PowerShell を再起動してください"
}

# ── Step 3: Node.js ───────────────────────────────────────────

Write-Step "Step 3/4: Node.js LTS の確認"

if (Test-CommandExists "node") {
    $nodeVersion = node --version
    Write-Success "Node.js は既にインストール済みです: $nodeVersion"
} else {
    Write-Info "Node.js LTS をインストールします..."
    winget install OpenJS.NodeJS.LTS --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Node.js のインストールに失敗しました。" -ForegroundColor Red
        Write-Host "  手動でインストールしてください: https://nodejs.org/" -ForegroundColor Yellow
        exit 1
    }

    # PATH を反映
    $nodePath = "$env:ProgramFiles\nodejs"
    if (Test-Path $nodePath) {
        $env:PATH = "$nodePath;$env:PATH"
    }

    Write-Success "Node.js LTS をインストールしました"
}

# ── Step 4: Tauri CLI ─────────────────────────────────────────

Write-Step "Step 4/4: Tauri CLI v2 の確認"

$tauriInstalled = $false
try {
    $tauriVersion = cargo tauri --version 2>$null
    if ($tauriVersion -match "tauri-cli 2") {
        $tauriInstalled = $true
    }
} catch {}

if ($tauriInstalled) {
    Write-Success "Tauri CLI は既にインストール済みです: $tauriVersion"
} else {
    if (-not (Test-CommandExists "cargo")) {
        Write-Host ""
        Write-Host "[WARNING] cargo が見つかりません。" -ForegroundColor Yellow
        Write-Host "  PowerShell を再起動して、再度このスクリプトを実行するか、" -ForegroundColor Yellow
        Write-Host "  手動で以下を実行してください:" -ForegroundColor Yellow
        Write-Host "    cargo install tauri-cli --version `"^2.0`"" -ForegroundColor White
    } else {
        Write-Info "Tauri CLI v2 をインストールします（初回は数分かかります）..."
        cargo install tauri-cli --version "^2.0"
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[ERROR] Tauri CLI のインストールに失敗しました。" -ForegroundColor Red
            exit 1
        }
        Write-Success "Tauri CLI v2 をインストールしました"
    }
}

# ── 完了メッセージ ────────────────────────────────────────────

Write-Host ""
Write-Host "=============================================" -ForegroundColor Green
Write-Host "  セットアップ完了！" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Green
Write-Host ""
Write-Host "次のステップ:" -ForegroundColor Cyan
Write-Host "  1. PowerShell を再起動して PATH を反映させる"
Write-Host "  2. プロジェクトルートで以下を実行:"
Write-Host ""
Write-Host "     npm install" -ForegroundColor White
Write-Host "     cargo tauri dev" -ForegroundColor White
Write-Host ""
Write-Host "  初回ビルドは Rust クレートのコンパイルで 5〜10 分かかる場合があります。" -ForegroundColor Gray
Write-Host ""
