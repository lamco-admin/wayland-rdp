# FreeRDP Windows Compilation Guide

**Purpose:** Build FreeRDP from source on Windows with full debug logging to get detailed RDP protocol diagnostics.

**Benefit:** FreeRDP provides much more verbose logging than Windows mstsc.exe, especially for protocol negotiation, TLS handshake, and codec details.

---

## Prerequisites

### 1. Install Visual Studio 2022 (Community Edition - Free)

**Download:** https://visualstudio.microsoft.com/downloads/

**Required Components:**
- Desktop development with C++
- Windows 10/11 SDK
- CMake tools for Windows
- Git for Windows

**Installation Steps:**
1. Run Visual Studio Installer
2. Select "Desktop development with C++"
3. In "Individual components" tab, ensure these are checked:
   - MSVC v143 - VS 2022 C++ x64/x86 build tools
   - Windows 10 SDK (latest version)
   - C++ CMake tools for Windows
   - Git for Windows
4. Click Install (requires ~8-10 GB)

### 2. Install Additional Tools

**CMake (if not included with VS):**
- Download: https://cmake.org/download/
- Get "Windows x64 Installer"
- During install, select "Add CMake to system PATH"

**Git (if not included with VS):**
- Download: https://git-scm.com/download/win
- Use default settings during installation

---

## Build FreeRDP from Source

### Step 1: Open Developer Command Prompt

1. Press Windows key
2. Type "Developer Command Prompt for VS 2022"
3. Run as Administrator (right-click → Run as administrator)

### Step 2: Create Build Directory

```cmd
cd C:\
mkdir FreeRDP-Build
cd FreeRDP-Build
```

### Step 3: Clone FreeRDP Repository

```cmd
git clone https://github.com/FreeRDP/FreeRDP.git
cd FreeRDP
```

**Check latest stable version:**
```cmd
git tag --list
```

**Recommended: Use stable release (e.g., 3.0.0):**
```cmd
git checkout 3.0.0
```

Or use latest development:
```cmd
git checkout master
```

### Step 4: Create Build Directory

```cmd
mkdir build
cd build
```

### Step 5: Configure with CMake (Debug Build)

**For maximum logging and debugging:**

```cmd
cmake .. ^
  -G "Visual Studio 17 2022" ^
  -A x64 ^
  -DCMAKE_BUILD_TYPE=Debug ^
  -DWITH_DEBUG_ALL=ON ^
  -DWITH_VERBOSE=ON ^
  -DWITH_VERBOSE_WINPR_ASSERT=ON ^
  -DWITH_CLIENT=ON ^
  -DWITH_SERVER=OFF ^
  -DBUILD_SHARED_LIBS=ON ^
  -DCMAKE_INSTALL_PREFIX=C:\FreeRDP-Install
```

**CMake Options Explained:**
- `-G "Visual Studio 17 2022"` - Use VS 2022 compiler
- `-A x64` - Build 64-bit version
- `-DCMAKE_BUILD_TYPE=Debug` - Debug build with symbols
- `-DWITH_DEBUG_ALL=ON` - Enable all debug logging
- `-DWITH_VERBOSE=ON` - Verbose output
- `-DWITH_CLIENT=ON` - Build client (xfreerdp)
- `-DWITH_SERVER=OFF` - Don't build server (we only need client)
- `-CMAKE_INSTALL_PREFIX` - Install location

**Expected Output:**
```
-- Configuring done
-- Generating done
-- Build files have been written to: C:/FreeRDP-Build/FreeRDP/build
```

### Step 6: Build FreeRDP

**Build command:**
```cmd
cmake --build . --config Debug --parallel
```

**This will take 10-30 minutes depending on your CPU.**

**Progress indicators:**
```
[1%] Building C object ...
[5%] Building C object ...
...
[100%] Built target xfreerdp-client
```

### Step 7: Install FreeRDP

```cmd
cmake --install . --config Debug
```

**This installs to:** `C:\FreeRDP-Install\`

### Step 8: Add to PATH (Optional)

**Temporary (current session only):**
```cmd
set PATH=C:\FreeRDP-Install\bin;%PATH%
```

**Permanent (via System Properties):**
1. Press Windows + R
2. Type `sysdm.cpl` and press Enter
3. Go to "Advanced" tab
4. Click "Environment Variables"
5. Under "User variables", select "Path"
6. Click "Edit"
7. Click "New"
8. Add: `C:\FreeRDP-Install\bin`
9. Click OK on all dialogs

---

## Using FreeRDP for Testing

### Basic Connection with Logging

```cmd
cd C:\FreeRDP-Install\bin

xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE
```

**Common Options:**

```cmd
xfreerdp.exe ^
  /v:192.168.10.205:3389 ^
  /cert:ignore ^
  /log-level:TRACE ^
  /log-filters:com.freerdp.* ^
  /size:1920x1080 ^
  /bpp:32
```

**Log Levels (most to least verbose):**
- `TRACE` - Everything (most detailed)
- `DEBUG` - Debug information
- `INFO` - Informational messages
- `WARN` - Warnings only
- `ERROR` - Errors only
- `FATAL` - Fatal errors only

### Capture Logs to File

**Method 1: Redirect to File**
```cmd
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE > rdp-session.log 2>&1
```

**Method 2: Use PowerShell**
```powershell
& "C:\FreeRDP-Install\bin\xfreerdp.exe" /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE *>&1 | Tee-Object -FilePath rdp-session.log
```

### Advanced Debugging Options

**Enable specific debug categories:**
```cmd
xfreerdp.exe ^
  /v:192.168.10.205:3389 ^
  /cert:ignore ^
  /log-level:DEBUG ^
  /log-filters:"core.nego,core.connection,core.activation" ^
  /sec:tls ^
  /rfx
```

**Test different security protocols:**
```cmd
# TLS only
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /sec:tls /log-level:TRACE

# RDP security (no TLS)
xfreerdp.exe /v:192.168.10.205:3389 /sec:rdp /log-level:TRACE

# NLA (Network Level Authentication)
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /sec:nla /u:username /p:password /log-level:TRACE
```

**Test different codecs:**
```cmd
# RemoteFX
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /rfx /log-level:TRACE

# H264 (if supported)
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /gfx:rfx /log-level:TRACE

# Raw bitmap (no compression)
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /bitmap-cache /log-level:TRACE
```

---

## Troubleshooting Build Issues

### Issue: CMake not found

**Solution:**
```cmd
# Add CMake to PATH manually
set PATH=C:\Program Files\CMake\bin;%PATH%
```

Or reinstall with "Add to PATH" option.

### Issue: Git not found

**Solution:**
```cmd
# Add Git to PATH
set PATH=C:\Program Files\Git\cmd;%PATH%
```

### Issue: MSVC compiler not found

**Error:** `error: Microsoft Visual Studio ... not found`

**Solution:**
1. Run from "Developer Command Prompt for VS 2022" (not regular cmd)
2. Or manually run: `"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"`

### Issue: OpenSSL missing

FreeRDP requires OpenSSL. CMake will try to find it automatically.

**If CMake can't find OpenSSL:**

1. Download pre-built binaries:
   - https://slproweb.com/products/Win32OpenSSL.html
   - Get "Win64 OpenSSL v3.x.x" (not Light version)

2. Install to default location: `C:\Program Files\OpenSSL-Win64\`

3. Add to CMake command:
```cmd
cmake .. ^
  -DOPENSSL_ROOT_DIR="C:\Program Files\OpenSSL-Win64" ^
  [... other options ...]
```

### Issue: Build fails with errors

**Common fixes:**

1. **Clean and rebuild:**
```cmd
cd C:\FreeRDP-Build\FreeRDP\build
del CMakeCache.txt
cmake .. [... your options ...]
cmake --build . --config Debug --parallel
```

2. **Use specific Visual Studio version:**
```cmd
cmake .. -G "Visual Studio 17 2022" [...]
```

3. **Build without parallelization:**
```cmd
cmake --build . --config Debug
```

---

## Quick Build Script

Save this as `build-freerdp-debug.bat`:

```batch
@echo off
REM FreeRDP Debug Build Script for Windows

echo ========================================
echo FreeRDP Debug Build Script
echo ========================================
echo.

REM Set paths
set BUILD_DIR=C:\FreeRDP-Build
set INSTALL_DIR=C:\FreeRDP-Install

REM Create directories
if not exist %BUILD_DIR% mkdir %BUILD_DIR%
cd /d %BUILD_DIR%

REM Clone or update repository
if not exist FreeRDP (
    echo Cloning FreeRDP repository...
    git clone https://github.com/FreeRDP/FreeRDP.git
) else (
    echo Updating FreeRDP repository...
    cd FreeRDP
    git pull
    cd ..
)

cd FreeRDP

REM Checkout stable version (optional)
REM git checkout 3.0.0

REM Create build directory
if not exist build mkdir build
cd build

REM Configure with CMake
echo.
echo Configuring with CMake...
cmake .. ^
  -G "Visual Studio 17 2022" ^
  -A x64 ^
  -DCMAKE_BUILD_TYPE=Debug ^
  -DWITH_DEBUG_ALL=ON ^
  -DWITH_VERBOSE=ON ^
  -DWITH_CLIENT=ON ^
  -DWITH_SERVER=OFF ^
  -DBUILD_SHARED_LIBS=ON ^
  -DCMAKE_INSTALL_PREFIX=%INSTALL_DIR%

if errorlevel 1 (
    echo CMake configuration failed!
    pause
    exit /b 1
)

REM Build
echo.
echo Building FreeRDP (this may take 10-30 minutes)...
cmake --build . --config Debug --parallel

if errorlevel 1 (
    echo Build failed!
    pause
    exit /b 1
)

REM Install
echo.
echo Installing FreeRDP to %INSTALL_DIR%...
cmake --install . --config Debug

if errorlevel 1 (
    echo Installation failed!
    pause
    exit /b 1
)

echo.
echo ========================================
echo Build Complete!
echo ========================================
echo.
echo FreeRDP installed to: %INSTALL_DIR%
echo Executable: %INSTALL_DIR%\bin\xfreerdp.exe
echo.
echo To test:
echo   cd %INSTALL_DIR%\bin
echo   xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE
echo.
pause
```

**Usage:**
1. Save as `C:\build-freerdp-debug.bat`
2. Run "Developer Command Prompt for VS 2022" as Administrator
3. Execute: `C:\build-freerdp-debug.bat`

---

## Testing Against wrd-server

### Connection Test with Full Logging

```cmd
cd C:\FreeRDP-Install\bin

xfreerdp.exe ^
  /v:192.168.10.205:3389 ^
  /cert:ignore ^
  /log-level:TRACE ^
  /log-filters:* ^
  /size:1280x800 ^
  /bpp:32 ^
  /compression ^
  /rfx ^
  /sec:tls > wrd-test.log 2>&1
```

**This will log:**
- TLS handshake details
- Certificate validation
- Capability exchange
- Codec negotiation
- Authentication sequence
- Channel setup
- Graphics updates
- Input events

### Compare with mstsc.exe

**Test 1: FreeRDP**
```cmd
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE > freerdp.log 2>&1
```

**Test 2: Windows RDP**
```cmd
mstsc.exe /v:192.168.10.205:3389
# No detailed logs available
```

FreeRDP logs will show exactly where protocol negotiation succeeds or fails.

---

## Log Analysis

### What to Look For

**Successful connection shows:**
```
[INFO] - Establishing connection to 192.168.10.205:3389
[DEBUG] - TCP connection established
[DEBUG] - Starting TLS handshake
[DEBUG] - TLS handshake successful
[DEBUG] - Negotiating RDP protocol
[INFO] - Protocol negotiation successful
[DEBUG] - Sending MCS Connect Initial
[DEBUG] - Received MCS Connect Response
[INFO] - Connection established successfully
```

**Failed connection shows exact failure point:**
```
[ERROR] - TLS handshake failed: <specific error>
[ERROR] - Protocol negotiation failed: <reason>
[ERROR] - Server refused connection: <code>
```

### Key Log Sections

1. **Connection Establishment**
   - Look for: "TCP connection established"

2. **TLS Handshake**
   - Look for: "TLS handshake successful" or error details

3. **Protocol Negotiation**
   - Look for: "Protocol negotiation successful"
   - Or: "Unsupported protocol" / "Protocol mismatch"

4. **Capability Exchange**
   - Look for: "Client capabilities" / "Server capabilities"
   - Shows what features are supported

5. **Graphics Pipeline**
   - Look for: "RemoteFX", "RFX", "H264", "Bitmap"
   - Shows codec negotiation

6. **Input Channel**
   - Look for: "Virtual channel" / "Input channel"
   - Shows mouse/keyboard setup

---

## Comparison: FreeRDP vs mstsc.exe

| Feature | FreeRDP (Debug) | mstsc.exe |
|---------|-----------------|-----------|
| **Logging** | Full TRACE logs | Minimal error codes |
| **Protocol Details** | Complete handshake info | None |
| **TLS Debug** | Certificate chains, cipher info | Only "certificate error" |
| **Codec Info** | Detailed codec negotiation | None |
| **Source Code** | Available to modify | Closed source |
| **Command Line** | Extensive options | Limited |
| **Cross-Platform** | Yes (Windows, Linux, Mac) | Windows only |

---

## Alternative: Pre-Built FreeRDP Binaries

If you don't want to compile from source, you can try pre-built binaries:

**WinPR/FreeRDP Nightly Builds:**
- https://ci.freerdp.com/
- Download latest successful build
- Extract and run

**Note:** Pre-built may not have all debug symbols, but still provides better logging than mstsc.exe.

---

## Using FreeRDP on Linux (Bonus)

If you also have a Linux machine, FreeRDP is much easier to install:

**Ubuntu/Debian:**
```bash
sudo apt install freerdp2-x11

# Test from Linux
xfreerdp /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE
```

**Fedora:**
```bash
sudo dnf install freerdp

xfreerdp /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE
```

---

## Next Steps After Building

1. **Test basic connection:**
```cmd
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /log-level:INFO
```

2. **If successful, enable full logging:**
```cmd
xfreerdp.exe /v:192.168.10.205:3389 /cert:ignore /log-level:TRACE > full-session.log 2>&1
```

3. **Compare logs with server logs:**
   - Server log: `wrd-server -c config.toml -vv --log-file server.log`
   - Client log: `xfreerdp.exe ... > client.log 2>&1`
   - Analyze both side-by-side

4. **Test specific features:**
   - Mouse/keyboard input
   - Clipboard sync
   - Different codecs
   - Multiple connections

---

## Useful FreeRDP Options Reference

```cmd
# Connection
/v:<server>:<port>              # Server address
/cert:ignore                    # Ignore certificate warnings
/cert:tofu                      # Trust on first use

# Authentication
/u:<username>                   # Username
/p:<password>                   # Password
/d:<domain>                     # Domain

# Display
/size:<WxH>                     # Window size (e.g., /size:1920x1080)
/f                              # Fullscreen
/monitors:<id>                  # Use specific monitor
/multimon                       # Use all monitors
/bpp:<depth>                    # Color depth (16, 24, 32)

# Security
/sec:rdp                        # RDP security
/sec:tls                        # TLS security
/sec:nla                        # Network Level Authentication

# Codecs
/rfx                            # RemoteFX codec
/gfx:rfx                        # Graphics RemoteFX
/gfx:avc420                     # H264 codec
/gdi:sw                         # Software GDI
/gdi:hw                         # Hardware GDI

# Logging
/log-level:<level>              # TRACE, DEBUG, INFO, WARN, ERROR, FATAL
/log-filters:<filters>          # Specific log categories

# Performance
/compression                    # Enable compression
/compression-level:<level>      # 0 (none) to 2 (max)
/network:auto                   # Auto-detect network
/network:modem                  # Optimize for slow link
/network:lan                    # Optimize for LAN

# Input
/kbd:0x00000409                 # Keyboard layout (US English)
/mouse-motion                   # Send mouse motion events
/multitouch                     # Enable multi-touch

# Clipboard
/clipboard                      # Enable clipboard redirection

# Audio
/sound                          # Enable sound redirection
/microphone                     # Enable microphone redirection

# Drives
/drive:share,/path/to/local     # Share local directory

# Smart Cards
/smartcard                      # Enable smart card redirection
```

---

## Support and Documentation

**Official Documentation:**
- https://github.com/FreeRDP/FreeRDP/wiki
- https://github.com/FreeRDP/FreeRDP/wiki/CommandLineInterface

**Build Documentation:**
- https://github.com/FreeRDP/FreeRDP/wiki/Compilation

**Issue Tracker:**
- https://github.com/FreeRDP/FreeRDP/issues

**IRC Channel:**
- #freerdp on irc.freenode.net

---

**End of Guide**

This guide provides everything needed to compile FreeRDP on Windows with full debug capabilities for testing wrd-server!
