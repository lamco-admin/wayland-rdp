#!/bin/bash
# Deploy lamco-rdp-server to RHEL 9 for testing
# This copies all required source code and builds on RHEL 9 with glibc 2.34

set -e

RHEL_HOST="greg@192.168.10.6"
RHEL_PASS="Bibi4189"
REMOTE_DIR="/home/greg/wayland-build"

echo "════════════════════════════════════════════════════════════"
echo "  Deploying lamco-rdp-server to RHEL 9"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "Target: $RHEL_HOST"
echo "Remote directory: $REMOTE_DIR"
echo ""

# Check if sshpass is available
if ! command -v sshpass &> /dev/null; then
    echo "ERROR: sshpass not found. Install with: sudo apt install sshpass"
    exit 1
fi

echo "Step 1: Creating directory structure on RHEL 9..."
sshpass -p "$RHEL_PASS" ssh "$RHEL_HOST" "mkdir -p $REMOTE_DIR"

echo ""
echo "Step 2: Syncing source directories (excluding target/)..."
echo ""

# Sync wrd-server-specs (main project)
echo "  → wrd-server-specs (1.7GB source)..."
rsync -avz --progress \
    --exclude 'target/' \
    --exclude '*.log' \
    --exclude '.git/' \
    --exclude '.claude/' \
    --exclude 'archive/' \
    -e "sshpass -p $RHEL_PASS ssh" \
    /home/greg/wayland/wrd-server-specs/ \
    "$RHEL_HOST:$REMOTE_DIR/wrd-server-specs/"

# Sync IronRDP fork
echo ""
echo "  → IronRDP fork (35MB source)..."
rsync -avz --progress \
    --exclude 'target/' \
    --exclude '.git/' \
    --exclude 'EGFX-*.md' \
    --exclude '.claude/' \
    -e "sshpass -p $RHEL_PASS ssh" \
    /home/greg/wayland/IronRDP/ \
    "$RHEL_HOST:$REMOTE_DIR/IronRDP/"

# Sync lamco-wayland (for lamco-portal)
echo ""
echo "  → lamco-wayland (2.8MB source)..."
rsync -avz --progress \
    --exclude 'target/' \
    --exclude '.git/' \
    -e "sshpass -p $RHEL_PASS ssh" \
    /home/greg/wayland/lamco-wayland/ \
    "$RHEL_HOST:$REMOTE_DIR/lamco-wayland/"

# Sync lamco-rdp-workspace (for pipewire, clipboard)
echo ""
echo "  → lamco-rdp-workspace (3.0MB source)..."
rsync -avz --progress \
    --exclude 'target/' \
    --exclude '.git/' \
    -e "sshpass -p $RHEL_PASS ssh" \
    /home/greg/wayland/lamco-rdp-workspace/ \
    "$RHEL_HOST:$REMOTE_DIR/lamco-rdp-workspace/"

# Sync openh264-rs (for VUI support)
echo ""
echo "  → openh264-rs (31MB source)..."
rsync -avz --progress \
    --exclude 'target/' \
    --exclude '.git/' \
    -e "sshpass -p $RHEL_PASS ssh" \
    /home/greg/openh264-rs/ \
    "$RHEL_HOST:$REMOTE_DIR/openh264-rs/"

echo ""
echo "Step 3: Creating build script on RHEL 9..."

# Create build script on remote
sshpass -p "$RHEL_PASS" ssh "$RHEL_HOST" "cat > $REMOTE_DIR/build.sh" <<'EOF'
#!/bin/bash
# Build lamco-rdp-server on RHEL 9

cd ~/wayland-build/wrd-server-specs

echo "Building lamco-rdp-server for RHEL 9..."
echo "This may take 5-15 minutes..."
echo ""

# Build release binary
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "  Build successful!"
    echo "════════════════════════════════════════════════════════════"
    echo ""
    echo "Binary location: target/release/lamco-rdp-server"
    echo ""

    # Check binary info
    ls -lh target/release/lamco-rdp-server
    file target/release/lamco-rdp-server
    ldd target/release/lamco-rdp-server | grep libc

    echo ""
    echo "To test:"
    echo "  cd ~/wayland-build/wrd-server-specs"
    echo "  ./target/release/lamco-rdp-server --help"
else
    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "  Build FAILED"
    echo "════════════════════════════════════════════════════════════"
    exit 1
fi
EOF

sshpass -p "$RHEL_PASS" ssh "$RHEL_HOST" "chmod +x $REMOTE_DIR/build.sh"

echo ""
echo "════════════════════════════════════════════════════════════"
echo "  Deployment complete!"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "Next steps:"
echo ""
echo "  1. SSH to RHEL 9:"
echo "     ssh $RHEL_HOST"
echo "     (password: $RHEL_PASS)"
echo ""
echo "  2. Build:"
echo "     cd $REMOTE_DIR"
echo "     ./build.sh"
echo ""
echo "  3. Test:"
echo "     cd wrd-server-specs"
echo "     ./target/release/lamco-rdp-server --help"
echo "     ./target/release/lamco-rdp-server --show-capabilities"
echo ""
echo "Build time estimate: 5-15 minutes (first build)"
echo ""
