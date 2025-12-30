#!/bin/bash
# Apply IronRDP patches before building

set -e

echo "Applying IronRDP patches..."

# Find ironrdp-cliprdr git checkout
CLIPRDR_SRC=$(find ~/.cargo/git/checkouts/ironrdp-*/*/crates/ironrdp-cliprdr/src -name "lib.rs" 2>/dev/null | head -1)

if [ -z "$CLIPRDR_SRC" ]; then
    echo "ERROR: ironrdp-cliprdr git checkout not found!"
    echo "Run 'cargo fetch' first to download dependencies"
    exit 1
fi

CLIPRDR_DIR=$(dirname "$CLIPRDR_SRC")
echo "Found ironrdp-cliprdr at: $CLIPRDR_DIR"

# Apply patch
if [ -f "$CLIPRDR_DIR/lib.rs.backup" ]; then
    echo "Restoring original lib.rs from backup..."
    cp "$CLIPRDR_DIR/lib.rs.backup" "$CLIPRDR_DIR/lib.rs"
fi

# Backup original
cp "$CLIPRDR_DIR/lib.rs" "$CLIPRDR_DIR/lib.rs.backup"

# Apply patch
echo "Applying server initiate_copy patch..."
cd "$(dirname "$CLIPRDR_DIR")"
patch -p1 < /home/greg/wayland/wrd-server-specs/patches/ironrdp-cliprdr-server-initiate-copy.patch

echo "✅ Patches applied successfully"
echo "Now run: cargo build --release"
