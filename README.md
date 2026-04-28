<div align="center">
  <img src="https://raw.githubusercontent.com/tauri-apps/tauri/HEAD/app-icon.png" width="120" alt="Logo" />
  <h1>🌐 Network Chat</h1>
  <p>
    <strong>A high-performance, Zero-Config Peer-to-Peer local network chat application.</strong>
  </p>
  <p>
    Built with 🦀 <b>Rust</b>, 🖥️ <b>Tauri</b>, and ⚡ <b>Vite</b>.
  </p>
</div>

---

## ✨ Features

- **Zero-Configuration Discovery**: Automatically detects other users on your local network using `mDNS` (Multicast DNS). No IP addresses or manual pairing required!
- **Blazing Fast P2P Messaging**: Powered by asynchronous Rust (`tokio`) TCP streams for instant communication.
- **Large File Transfers**: Easily share photos, documents, and large files. Streams directly between peers into your `Downloads` folder, bypassing memory bloat.
- **Modern Premium UI**: Beautiful dark-mode interface featuring glassmorphism, dynamic transitions, and real-time reactivity built with Vanilla JS and CSS.
- **Self-Healing Sync**: Automatic background polling ensures you never miss a peer, even if UDP multicast packets are dropped over flaky Wi-Fi.

## 🛠️ Tech Stack

- **Core Framework**: [Tauri v2](https://tauri.app/)
- **Backend**: Rust 🦀 (`tokio`, `mdns-sd`, `serde`)
- **Frontend**: Vite ⚡ (Vanilla HTML/CSS/JS)
- **Networking**: Custom TCP protocol over local network

## 🚀 Getting Started

### Prerequisites
- [Node.js](https://nodejs.org/)
- [Rust & Cargo](https://rustup.rs/)
- Native OS Dependencies (Linux only: `webkit2gtk`, `build-essential`, etc.)

*If you are on Debian/Ubuntu, you can install the system prerequisites easily using the provided script:*
```bash
./install_dependencies.sh
```

### Installation

This project includes a convenient `Makefile` to handle building, packaging, and installing the application on Debian-based systems.

**1. Clone the repository**
```bash
git clone https://github.com/yourusername/network-chat.git
cd network-chat
```

**2. Develop Locally**
To run the application in development mode with hot-reloading:
```bash
npm install
npm run tauri dev
```

**3. Build and Install (.deb Package)**
To compile a production-ready Debian package and install it system-wide:
```bash
make install
```
*(This command automatically runs `make build` and then uses `dpkg` and `apt` to install the `.deb` file).*

## 🧹 Make Commands
- `make build` - Packages the application into a `.deb` file without installing.
- `make install` - Builds the `.deb` and installs it on your system.
- `make uninstall` - Removes the installed `network-chat` application from your system.
- `make clean` - Deletes all build artifacts (`node_modules` and `src-tauri/target`).

## 🤝 Contributing
Contributions, issues, and feature requests are welcome! Feel free to check the [issues page](../../issues).

## 📝 License
This project is licensed under the MIT License.
