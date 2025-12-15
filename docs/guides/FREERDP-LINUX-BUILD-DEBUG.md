# FreeRDP Linux Build Guide - Debug Client

**Purpose:** Build FreeRDP from source with extensive debug logging to diagnose RDP connection issues.

**Benefit:** Get detailed protocol traces, TLS handshake info, codec negotiation details that mstsc.exe doesn't provide.

---

## Quick Build (Ubuntu/Debian)

### Install Dependencies

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    git \
    cmake \
    ninja-build \
    libssl-dev \
    libx11-dev \
    libxext-dev \
    libxinerama-dev \
    libxcursor-dev \
    libxdamage-dev \
    libxv-dev \
    libxkbfile-dev \
    libasound2-dev \
    libcups2-dev \
    libxml2 \
    libxml2-dev \
    libxrandr-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libxi-dev \
    libavutil-dev \
    libavcodec-dev \
    libxtst-dev \
    libgtk-3-dev \
    libgcrypt20-dev \
    libpulse-dev \
    libsystemd-dev \
    libpcsclite-dev \
    libwayland-dev \
    libxkbcommon-dev
```

### Clone and Build

```bash
cd ~/
git clone https://github.com/FreeRDP/FreeRDP.git
cd FreeRDP

# Use stable version
git checkout 3.10.2

# Create build directory
mkdir build && cd build

# Configure with debug logging
cmake .. \
    -DCMAKE_BUILD_TYPE=Debug \
    -DWITH_DEBUG_ALL=ON \
    -DWITH_VERBOSE=ON \
    -DCMAKE_INSTALL_PREFIX=/usr/local \
    -DBUILD_TESTING=OFF \
    -GNinja

# Build (use all cores)
ninja -j$(nproc)

# Install (optional, or run from build dir)
sudo ninja install
```

### Run with Maximum Logging

```bash
# From build directory
./client/X11/xfreerdp \
    /v:192.168.10.205:3389 \
    /cert:ignore \
    /u: \
    /p: \
    /sec:tls \
    /log-level:TRACE \
    /log-filters:"com.freerdp.*:TRACE" \
    +clipboard \
    2>&1 | tee freerdp-debug.log
```

**Log Options:**
- `/log-level:TRACE` - Most verbose
- `/log-level:DEBUG` - Detailed
- `/log-level:INFO` - Normal
- `/log-filters:"com.freerdp.*:TRACE"` - Filter specific components

**Useful Filters:**
- `com.freerdp.core.connection:TRACE` - Connection details
- `com.freerdp.core.activation:TRACE` - Activation sequence
- `com.freerdp.codec.remotefx:TRACE` - RemoteFX codec
- `com.freerdp.core.gcc:TRACE` - Capability negotiation

---

## Enhanced Logging on Server

While FreeRDP builds, let's add more detailed logging to the server to diagnose the "finalize" failure.

**What to log:**
1. Every stage of RDP connection handshake
2. Capability negotiation
3. Clipboard backend creation timing
4. Frame send success/failure with details
5. Channel state transitions

---

## Quick Alternative: Use System FreeRDP

If building takes too long:

```bash
# Install from package manager (less debug info but faster)
sudo apt-get install freerdp2-x11

# Run with verbose logging
xfreerdp /v:192.168.10.205:3389 /cert:ignore /log-level:DEBUG +clipboard
```
