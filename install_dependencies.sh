#!/bin/bash
set -e

echo "Updating package list..."
sudo apt-get update

echo "Installing Linux dependencies for Tauri (WebKit2GTK, etc.)..."
# Required dependencies for building Tauri v2 apps on Ubuntu/Debian
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

echo "Checking for Node.js/NPM..."
if ! command -v npm &> /dev/null; then
    echo "NPM not found. Installing Node.js..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
    sudo apt-get install -y nodejs
else
    echo "Node.js is already installed."
fi

echo "Checking for Rust/Cargo..."
if ! command -v cargo &> /dev/null; then
    echo "Cargo not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed."
fi

echo ""
echo "All dependencies installed successfully!"
echo "If this is your first time installing Rust, please run 'source $HOME/.cargo/env' or restart your terminal."
