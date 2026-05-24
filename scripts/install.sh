#!/usr/bin/env bash
# clip-win 開発環境セットアップスクリプト（Linux / macOS）
# ==========================================================
# 使い方:
#   bash scripts/install.sh
#
# インストールされるもの:
#   - Rust (rustup)
#   - Node.js (nvm 経由で LTS バージョン)
#   - Tauri CLI v2
#   - Linux の場合: GTK / WebKitGTK などの Tauri 依存パッケージ
#
# 注意:
#   clip-win は Windows 専用アプリです。
#   このスクリプトは Linux/macOS での「コード編集・cargo check」用の
#   開発環境を構築します。`cargo tauri dev` の動作確認は Windows が必要です。
# ==========================================================

set -euo pipefail

# ── カラー定義 ────────────────────────────────────────────────

GREEN='\033[0;32m'
CYAN='\033[0;36m'
GRAY='\033[0;37m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

step()    { echo -e "\n${CYAN}>>> $1${NC}"; }
success() { echo -e "  ${GREEN}✓ $1${NC}"; }
info()    { echo -e "  ${GRAY}• $1${NC}"; }
warn()    { echo -e "  ${YELLOW}⚠ $1${NC}"; }
error()   { echo -e "  ${RED}✗ $1${NC}"; exit 1; }

command_exists() { command -v "$1" &>/dev/null; }

# ── OS 検出 ───────────────────────────────────────────────────

OS="$(uname -s)"
case "$OS" in
    Linux*)  PLATFORM="linux" ;;
    Darwin*) PLATFORM="macos" ;;
    *)       error "未対応の OS: $OS" ;;
esac

echo ""
echo "================================================="
echo " clip-win 開発環境セットアップ ($PLATFORM)"
echo "================================================="

# ── Step 1: Linux の場合はシステム依存パッケージをインストール ───

if [ "$PLATFORM" = "linux" ]; then
    step "Step 1/4: Tauri の Linux 依存パッケージを確認"

    if command_exists apt-get; then
        info "apt でパッケージをインストールします..."
        sudo apt-get update -q
        sudo apt-get install -y \
            libgtk-3-dev \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf \
            libssl-dev \
            pkg-config \
            build-essential \
            curl \
            wget
        success "依存パッケージをインストールしました"
    elif command_exists dnf; then
        info "dnf でパッケージをインストールします..."
        sudo dnf install -y \
            gtk3-devel \
            webkit2gtk4.1-devel \
            libappindicator-gtk3-devel \
            librsvg2-devel \
            openssl-devel \
            pkg-config \
            gcc \
            curl \
            wget
        success "依存パッケージをインストールしました"
    elif command_exists pacman; then
        info "pacman でパッケージをインストールします..."
        sudo pacman -Sy --noconfirm \
            gtk3 \
            webkit2gtk-4.1 \
            libappindicator-gtk3 \
            librsvg \
            openssl \
            pkgconf \
            base-devel \
            curl \
            wget
        success "依存パッケージをインストールしました"
    else
        warn "パッケージマネージャーが検出できませんでした。"
        warn "Tauri の依存パッケージを手動でインストールしてください:"
        warn "https://tauri.app/start/prerequisites/"
    fi
else
    step "Step 1/4: macOS システムツールの確認"
    if ! command_exists xcode-select; then
        info "Xcode Command Line Tools をインストールします..."
        xcode-select --install || true
        success "Xcode Command Line Tools のインストールを開始しました"
    else
        success "Xcode Command Line Tools は既にインストール済みです"
    fi
fi

# ── Step 2: Rust ─────────────────────────────────────────────

step "Step 2/4: Rust (rustup) の確認"

if command_exists rustc; then
    RUST_VERSION="$(rustc --version)"
    success "Rust は既にインストール済みです: $RUST_VERSION"
    info "最新の stable に更新します..."
    rustup update stable
else
    info "Rust をインストールします..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
    success "Rust をインストールしました"
fi

# cargo の PATH を現在のシェルに反映
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

RUST_VERSION="$(rustc --version 2>/dev/null || echo 'バージョン確認失敗')"
info "Rust バージョン: $RUST_VERSION"

# ── Step 3: Node.js (nvm) ─────────────────────────────────────

step "Step 3/4: Node.js の確認"

if command_exists node; then
    NODE_VERSION="$(node --version)"
    success "Node.js は既にインストール済みです: $NODE_VERSION"
else
    if command_exists nvm; then
        info "nvm で Node.js LTS をインストールします..."
        nvm install --lts
        nvm use --lts
    else
        info "nvm をインストールします..."
        NVM_VERSION="0.40.1"
        curl -o- "https://raw.githubusercontent.com/nvm-sh/nvm/v${NVM_VERSION}/install.sh" | bash

        # nvm を現在のシェルに読み込む
        export NVM_DIR="$HOME/.nvm"
        # shellcheck source=/dev/null
        [ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

        info "Node.js LTS をインストールします..."
        nvm install --lts
        nvm use --lts
    fi

    NODE_VERSION="$(node --version 2>/dev/null || echo 'バージョン確認失敗')"
    success "Node.js をインストールしました: $NODE_VERSION"
fi

# ── Step 4: Tauri CLI ─────────────────────────────────────────

step "Step 4/4: Tauri CLI v2 の確認"

if cargo tauri --version 2>/dev/null | grep -q "tauri-cli 2"; then
    TAURI_VERSION="$(cargo tauri --version)"
    success "Tauri CLI は既にインストール済みです: $TAURI_VERSION"
else
    if ! command_exists cargo; then
        warn "cargo が見つかりません。シェルを再起動して再度このスクリプトを実行してください。"
        warn "または手動で以下を実行:"
        warn "  cargo install tauri-cli --version '^2.0'"
    else
        info "Tauri CLI v2 をインストールします（初回は数分かかります）..."
        cargo install tauri-cli --version "^2.0"
        success "Tauri CLI v2 をインストールしました"
    fi
fi

# ── 完了メッセージ ────────────────────────────────────────────

echo ""
echo -e "${GREEN}=============================================${NC}"
echo -e "${GREEN}  セットアップ完了！${NC}"
echo -e "${GREEN}=============================================${NC}"
echo ""
echo -e "${CYAN}次のステップ:${NC}"
echo "  1. シェルを再起動して PATH を反映させる（または: source ~/.bashrc / source ~/.zshrc）"
echo "  2. プロジェクトルートで以下を実行:"
echo ""
echo "     npm install"
echo "     cargo check --target x86_64-pc-windows-gnu  # Linux/macOS での構文チェック"
echo ""
echo -e "${YELLOW}注意: clip-win は Windows 専用です。${NC}"
echo -e "${YELLOW}  cargo tauri dev での動作確認は Windows 環境で行ってください。${NC}"
echo ""
