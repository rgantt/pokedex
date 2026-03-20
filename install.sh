#!/bin/sh
set -e

INSTALL_DIR="/usr/local/bin"
BINARY_NAME="pokedex"

echo "Building release binary..."
cargo build --release

echo "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo "Installed $(${BINARY_NAME} --version 2>/dev/null || echo "${BINARY_NAME}") to ${INSTALL_DIR}/${BINARY_NAME}"
