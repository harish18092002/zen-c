#!/usr/bin/env bash
set -euo pipefail

echo "==> Setting up Zen Mode development environment"

# Rust
if ! command -v rustup &>/dev/null; then
  echo "  Installing Rust via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi
echo "  Rust toolchain: $(rustup show active-toolchain 2>/dev/null || echo 'see rust-toolchain.toml')"

# pnpm
if ! command -v pnpm &>/dev/null; then
  echo "  Installing pnpm..."
  npm install -g pnpm@latest
fi
echo "  pnpm: $(pnpm --version)"

# Node dependencies
echo "==> Installing Node dependencies"
pnpm install

# Tauri prerequisites check
echo "==> Checking Tauri system prerequisites"
if command -v cargo &>/dev/null; then
  cargo tauri info 2>/dev/null || echo "  (tauri-cli not installed yet — run: cargo install tauri-cli)"
fi

echo ""
echo "==> Setup complete."
echo ""
echo "  Run the app in development:"
echo "    pnpm dev"
echo ""
echo "  Run Rust tests:"
echo "    cargo test --workspace"
echo ""
echo "  Run the diagnostic CLI:"
echo "    cargo run --bin zen-cli -- --help"
