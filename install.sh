#!/bin/sh
set -e

INSTALL_DIR="/usr/local/bin"
BINARY_NAME="pokedex"

# Resolve cargo from common install locations
if ! command -v cargo >/dev/null 2>&1; then
    [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
fi

echo "Building release binary..."
cargo build --release

echo "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
if [ -w "${INSTALL_DIR}" ]; then
    cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
else
    echo "(requires sudo)"
    sudo cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
fi

echo "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"
