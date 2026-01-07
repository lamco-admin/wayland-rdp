#!/bin/bash
# =============================================================================
# lamco-rdp-server Test Runner
# =============================================================================
#
# Usage: ./run-server.sh [options]
#
# Options:
#   --avc444     Enable AVC444 codec (for newer systems)
#   --trace      Enable trace-level logging
#   --caps       Show capabilities and exit
#   --help       Show this help
#
# Requirements:
#   - Run from inside a Wayland session (not SSH)
#   - XDG Desktop Portal must be available
#   - PipeWire must be running
#
# =============================================================================

set -e

# Get script directory (works even if called from elsewhere)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Configuration
BINARY="./lamco-rdp-server"
CONFIG="./config.toml"
LOG_DIR="./logs"
HOSTNAME=$(hostname -s)
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_FILE="${LOG_DIR}/${HOSTNAME}-${TIMESTAMP}.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
EXTRA_ARGS=""
SHOW_CAPS=false

for arg in "$@"; do
    case $arg in
        --avc444)
            echo -e "${YELLOW}Note: AVC444 requested - edit config.toml to enable${NC}"
            echo "Set avc444_enabled = true in [egfx] section"
            ;;
        --trace)
            # Would need to modify config or add env var support
            echo -e "${YELLOW}Trace logging: edit config.toml [logging] level = \"trace\"${NC}"
            ;;
        --caps)
            SHOW_CAPS=true
            ;;
        --help|-h)
            head -25 "$0" | tail -20
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
    esac
done

# Preflight checks
echo "=== lamco-rdp-server Test Runner ==="
echo ""

# Check binary exists
if [[ ! -x "$BINARY" ]]; then
    echo -e "${RED}Error: Binary not found or not executable: $BINARY${NC}"
    echo ""
    echo "Copy the binary from your build machine:"
    echo "  scp user@build-host:lamco-rdp-server/target/release/lamco-rdp-server ."
    exit 1
fi

# Check config exists
if [[ ! -f "$CONFIG" ]]; then
    echo -e "${RED}Error: Config not found: $CONFIG${NC}"
    exit 1
fi

# Check certs exist
if [[ ! -f "./certs/test-cert.pem" ]] || [[ ! -f "./certs/test-key.pem" ]]; then
    echo -e "${RED}Error: Certificates not found in ./certs/${NC}"
    exit 1
fi

# Check Wayland session
if [[ -z "$WAYLAND_DISPLAY" ]]; then
    echo -e "${RED}Error: Not running in a Wayland session${NC}"
    echo "This must be run from within a graphical Wayland session, not SSH."
    exit 1
fi

# Check XDG Portal
if ! dbus-send --session --print-reply --dest=org.freedesktop.portal.Desktop \
    /org/freedesktop/portal/desktop org.freedesktop.DBus.Peer.Ping &>/dev/null; then
    echo -e "${YELLOW}Warning: XDG Desktop Portal may not be available${NC}"
fi

# Check PipeWire
if ! command -v pw-cli &>/dev/null || ! pw-cli info &>/dev/null 2>&1; then
    echo -e "${YELLOW}Warning: PipeWire may not be running${NC}"
fi

# Ensure log directory exists
mkdir -p "$LOG_DIR"

# Show environment
echo "Environment:"
echo "  Hostname:  $HOSTNAME"
echo "  Wayland:   $WAYLAND_DISPLAY"
echo "  Desktop:   ${XDG_CURRENT_DESKTOP:-unknown}"
echo "  Config:    $CONFIG"
echo "  Log file:  $LOG_FILE"
echo ""

# Show capabilities only
if [[ "$SHOW_CAPS" == "true" ]]; then
    echo "Checking capabilities..."
    exec "$BINARY" --config "$CONFIG" --show-capabilities 2>&1 | tee "$LOG_FILE"
fi

# Run server
echo -e "${GREEN}Starting server...${NC}"
echo "Press Ctrl+C to stop"
echo ""
echo "Connect with: mstsc.exe /v:$(hostname -I | awk '{print $1}'):3389"
echo "=============================================="
echo ""

# Run with output to both console and log file
exec "$BINARY" --config "$CONFIG" 2>&1 | tee "$LOG_FILE"
