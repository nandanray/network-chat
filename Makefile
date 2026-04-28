.PHONY: build install uninstall clean

# Variables
APP_NAME = network-chat
VERSION = 0.1.0
DEB_FILE = src-tauri/target/release/bundle/deb/$(APP_NAME)_$(VERSION)_amd64.deb

build:
	@echo "Building Debian package..."
	npm install
	npm run tauri build

install: build
	@echo "Installing $(APP_NAME)..."
	sudo dpkg -i $(DEB_FILE)
	sudo apt-get install -f -y

uninstall:
	@echo "Uninstalling $(APP_NAME)..."
	sudo apt-get remove -y $(APP_NAME)

clean:
	@echo "Cleaning build directories..."
	rm -rf node_modules
	rm -rf src-tauri/target
	@echo "Clean complete!"
