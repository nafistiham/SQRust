#!/usr/bin/env sh
set -e

REPO="nafistiham/SQRust"
BIN="sqrust"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
      *)
        echo "Unsupported architecture: $ARCH"
        echo "Download a binary manually from: https://github.com/$REPO/releases"
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-apple-darwin" ;;
      arm64)   TARGET="aarch64-apple-darwin" ;;
      *)
        echo "Unsupported architecture: $ARCH"
        echo "Download a binary manually from: https://github.com/$REPO/releases"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS"
    echo "Download a binary manually from: https://github.com/$REPO/releases"
    exit 1
    ;;
esac

ARCHIVE="${BIN}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${ARCHIVE}"

echo "Downloading sqrust for ${TARGET}..."
curl -sSfL "$URL" | tar -xz -C /tmp

echo "Installing to ${INSTALL_DIR}/${BIN}..."
if [ -w "$INSTALL_DIR" ]; then
  mv "/tmp/${BIN}" "${INSTALL_DIR}/${BIN}"
else
  sudo mv "/tmp/${BIN}" "${INSTALL_DIR}/${BIN}"
fi

chmod +x "${INSTALL_DIR}/${BIN}"

echo ""
echo "sqrust installed successfully."
echo "Run: sqrust --version"
