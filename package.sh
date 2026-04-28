#!/bin/bash
set -e

echo "Installing Node.js dependencies..."
npm install

echo "Building and packaging the application via Tauri..."
# This command compiles the Rust backend, builds the Vite frontend, 
# and packages them into a Debian (.deb) file along with an AppImage.
npm run tauri build

echo ""
echo "Packaging complete!"
echo "You can find your generated Debian package (.deb) inside:"
echo "  src-tauri/target/release/bundle/deb/"
echo ""
echo "To install the package on any Debian-based system (like Ubuntu), copy the file and run:"
echo "  sudo dpkg -i src-tauri/target/release/bundle/deb/network-chat_0.1.0_amd64.deb"
echo "  sudo apt-get install -f   # (This ensures any missing libraries are automatically downloaded and installed)"
