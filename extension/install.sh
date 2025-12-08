#!/bin/bash
# Install wayland-rdp-clipboard extension locally for development/testing

set -e

EXTENSION_UUID="wayland-rdp-clipboard@wayland-rdp.io"
EXTENSION_DIR="$HOME/.local/share/gnome-shell/extensions/$EXTENSION_UUID"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Installing $EXTENSION_UUID..."

# Compile schemas
echo "Compiling GSettings schemas..."
glib-compile-schemas "$SCRIPT_DIR/schemas/"

# Create extension directory
echo "Creating extension directory..."
mkdir -p "$EXTENSION_DIR"

# Copy files
echo "Copying files..."
cp "$SCRIPT_DIR/extension.js" "$EXTENSION_DIR/"
cp "$SCRIPT_DIR/metadata.json" "$EXTENSION_DIR/"
cp -r "$SCRIPT_DIR/schemas" "$EXTENSION_DIR/"

echo "Extension installed to: $EXTENSION_DIR"

# Check if extension is already enabled
if gnome-extensions list --enabled 2>/dev/null | grep -q "$EXTENSION_UUID"; then
    echo "Extension is already enabled."
    echo "To apply changes, restart GNOME Shell:"
    echo "  - Wayland: Log out and log back in"
    echo "  - X11: Press Alt+F2, type 'r', press Enter"
else
    echo ""
    echo "To enable the extension, run:"
    echo "  gnome-extensions enable $EXTENSION_UUID"
    echo ""
    echo "Then restart GNOME Shell:"
    echo "  - Wayland: Log out and log back in"
    echo "  - X11: Press Alt+F2, type 'r', press Enter"
fi

echo ""
echo "To test, run:"
echo "  gdbus monitor --session --dest org.wayland_rdp.Clipboard --object-path /org/wayland_rdp/Clipboard"
echo ""
echo "Then copy some text and watch for ClipboardChanged signals."
