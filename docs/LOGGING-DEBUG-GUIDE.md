# Logging and Debugging Guide

**Purpose:** Guide for debugging lamco-rdp-server during development and testing
**Last Updated:** 2025-12-21
**Status:** Ready for KDE VM testing

---

## LOGGING INFRASTRUCTURE

### What We Use

**Logging Framework:** [`tracing`](https://docs.rs/tracing/) v0.1

**Why `tracing` instead of `log`:**
- Structured logging with spans and events
- Zero-cost when disabled
- Async-aware (works with tokio)
- Better performance than traditional logging
- Used by all our dependencies (IronRDP, tokio, ashpd)

### Components Using `tracing`

| Component | Crate | Logs As |
|-----------|-------|---------|
| Server code | lamco-rdp-server | `lamco_rdp_server` |
| Portal integration | lamco-portal | `lamco_portal` |
| PipeWire capture | lamco-pipewire | `lamco_pipewire` |
| Video processing | lamco-video | `lamco_video` |
| Input handling | lamco-rdp-input | `lamco_rdp_input` |
| Clipboard core | lamco-clipboard-core | `lamco_clipboard_core` |
| RDP clipboard | lamco-rdp-clipboard | `lamco_rdp_clipboard` |
| IronRDP server | ironrdp-server | `ironrdp_server` |
| IronRDP clipboard | ironrdp-cliprdr | `ironrdp_cliprdr` |
| Portal D-Bus | ashpd | `ashpd` |

---

## CURRENT CONFIGURATION

### Default Filter (`src/main.rs:96-102`)

```rust
// When NOT using RUST_LOG environment variable:
tracing_subscriber::EnvFilter::new(format!(
    "lamco_rdp_server={},ironrdp_cliprdr=trace,ironrdp_server=trace,warn",
    log_level  // info/debug/trace based on -v flags
))
```

**What this enables:**
- ✅ `lamco_rdp_server` at chosen level (info/debug/trace)
- ✅ `ironrdp_cliprdr` at trace (clipboard state machine debugging)
- ✅ `ironrdp_server` at trace (RDP protocol debugging)
- ✅ Everything else at warn or higher

### ⚠️ CRITICAL GAP: Published Crates Logging

**Problem:** Published `lamco-*` crates won't log unless explicitly enabled!

**Example:**
```bash
# Current default filter:
lamco_rdp_server=debug  # ✅ Server logs visible

# BUT these are filtered out:
lamco_portal=debug      # ❌ Portal logs hidden
lamco_pipewire=debug    # ❌ PipeWire logs hidden
lamco_video=debug       # ❌ Video logs hidden
```

**Why this matters for testing:**
- Portal session creation logs won't show
- PipeWire stream creation logs won't show
- Video frame processing logs won't show

---

## RECOMMENDED TESTING CONFIGURATIONS

### Configuration 1: Basic Testing (Default + lamco crates)

**Use when:** First connection attempt, basic functionality testing

```bash
RUST_LOG=lamco=info,ironrdp=info,warn ./target/release/lamco-rdp-server -c config.toml
```

**What you'll see:**
- ✅ All lamco-* crates at info level
- ✅ All ironrdp-* crates at info level
- ✅ Other crates at warn level

**Expected output:**
```
INFO lamco_portal: Initializing Portal connection
INFO lamco_portal: Portal session created successfully
INFO lamco_pipewire: PipeWire thread manager created
INFO lamco_pipewire: Stream 42 created successfully
INFO lamco_rdp_server::server: WRD Server initialized successfully
INFO lamco_rdp_server::server: Server is ready and listening
```

### Configuration 2: Detailed Debugging (All lamco + IronRDP at debug)

**Use when:** Connection issues, need to see detailed flow

```bash
RUST_LOG=lamco=debug,ironrdp=debug,warn ./target/release/lamco-rdp-server -c config.toml -vv
```

**What you'll see:**
- ✅ All lamco-* crates at debug level
- ✅ All ironrdp-* crates at debug level
- ✅ Portal API calls
- ✅ PipeWire frame capture
- ✅ RDP protocol negotiation

### Configuration 3: Maximum Verbosity (Everything at trace)

**Use when:** Debugging crash, state machine issue, or mysterious bug

```bash
RUST_LOG=trace,ironrdp=trace ./target/release/lamco-rdp-server -c config.toml -vvv 2>&1 | tee debug-full.log
```

**What you'll see:**
- ✅ Every function call
- ✅ Every frame processed
- ✅ Every Portal D-Bus call
- ✅ Every IronRDP packet
- ⚠️ **WARNING:** Extremely verbose (hundreds of lines/second)

### Configuration 4: Module-Specific Debugging

**Use when:** Debugging specific subsystem

**Portal issues:**
```bash
RUST_LOG=lamco_portal=trace,lamco_rdp_server=debug,warn
```

**PipeWire issues:**
```bash
RUST_LOG=lamco_pipewire=trace,lamco_rdp_server=debug,warn
```

**Video pipeline issues:**
```bash
RUST_LOG=lamco_video=trace,lamco_rdp_server::server::display_handler=trace,warn
```

**Input injection issues:**
```bash
RUST_LOG=lamco_rdp_input=trace,lamco_portal::remote_desktop=trace,warn
```

**Clipboard issues:**
```bash
RUST_LOG=lamco_clipboard_core=trace,lamco_rdp_clipboard=trace,ironrdp_cliprdr=trace,warn
```

---

## CLI FLAGS FOR LOGGING

### Verbosity Flags

```bash
# No flags = INFO level (main.rs:91)
./lamco-rdp-server -c config.toml

# -v = DEBUG level (main.rs:92)
./lamco-rdp-server -c config.toml -v

# -vv or more = TRACE level (main.rs:93)
./lamco-rdp-server -c config.toml -vv
```

**Note:** These flags ONLY affect `lamco_rdp_server` target, not dependencies!

### Log Format Options

```bash
# Pretty format (default) - human readable, color
./lamco-rdp-server -c config.toml

# Compact format - one line per event
./lamco-rdp-server -c config.toml --log-format compact

# JSON format - structured, machine parseable
./lamco-rdp-server -c config.toml --log-format json
```

### Log to File

```bash
# Log to file AND stdout
./lamco-rdp-server -c config.toml --log-file server.log

# JSON to file for analysis
./lamco-rdp-server -c config.toml --log-format json --log-file server-$(date +%Y%m%d-%H%M%S).json
```

---

## RECOMMENDED TEST COMMAND (KDE VM)

### First Connection Attempt

```bash
RUST_LOG=lamco=info,ironrdp=info,ashpd=debug,warn \
  ./target/release/lamco-rdp-server \
  -c config.toml \
  -vv \
  --log-file kde-test-$(date +%Y%m%d-%H%M%S).log
```

**Why this configuration:**
- `lamco=info` - See all our code's important events
- `ironrdp=info` - See RDP protocol events
- `ashpd=debug` - See Portal API calls (critical for debugging permissions)
- `warn` - Suppress noise from other crates
- `-vv` - Enable trace for lamco_rdp_server specifically
- `--log-file` - Save for later analysis

### If Connection Fails

```bash
RUST_LOG=trace,h2=warn,hyper=warn \
  ./target/release/lamco-rdp-server \
  -c config.toml \
  -vvv \
  --log-file kde-fail-$(date +%Y%m%d-%H%M%S).log
```

**Why:**
- `trace` - See everything
- `h2=warn,hyper=warn` - Suppress HTTP/2 noise (from ashpd D-Bus)
- Save full trace for debugging

---

## WHAT TO LOOK FOR IN LOGS

### Successful Startup Sequence

```
INFO lamco_rdp_server: Starting WRD-Server v0.1.0
INFO lamco_rdp_server::utils: System diagnostics: ...
INFO lamco_rdp_server::config: Configuration loaded successfully
INFO lamco_rdp_server::server: Initializing WRD Server
INFO lamco_rdp_server::server: Setting up Portal connection
INFO lamco_portal: Portal manager created
INFO lamco_portal: Creating combined portal session
INFO lamco_portal::clipboard: Portal Clipboard created
INFO lamco_portal: Portal session created successfully (clipboard enabled if available)
INFO lamco_portal: Portal session started with 1 streams, PipeWire FD: 15
INFO lamco_rdp_server::server: Initial desktop size: 1920x1080
INFO lamco_rdp_server::server: Display handler created
INFO lamco_pipewire: PipeWire thread manager created
DEBUG lamco_pipewire: Creating stream for node 42
INFO lamco_pipewire: Stream 42 created successfully
INFO lamco_rdp_server::server: Graphics drain task started
INFO lamco_rdp_server::server::display_handler: Starting display update pipeline task
INFO lamco_rdp_server::server: Creating input handler
INFO lamco_rdp_server::server: Input handler created successfully
INFO lamco_rdp_server::server: Full multiplexer drain loop started
INFO lamco_rdp_server::server: Initializing clipboard manager
INFO lamco_rdp_server::server: Building IronRDP server
INFO lamco_rdp_server::server: WRD Server initialized successfully
INFO lamco_rdp_server::server: ╔════════════════════════════════════╗
INFO lamco_rdp_server::server: ║  WRD-Server is Starting            ║
INFO lamco_rdp_server::server: ╚════════════════════════════════════╝
INFO lamco_rdp_server::server: Server is ready and listening
INFO lamco_rdp_server::server: Waiting for clients to connect...
```

### Client Connection

```
INFO ironrdp_server: New connection from 192.168.1.100:54321
DEBUG ironrdp_server: TLS handshake in progress
INFO ironrdp_server: TLS handshake successful
DEBUG ironrdp_server: RDP capability negotiation
INFO ironrdp_server: Client connected successfully
INFO lamco_rdp_server::server::display_handler: Processing frame 1
```

### Portal Permission Dialog

Look for Portal session creation - if this fails, user didn't grant permission:

```
INFO lamco_portal: Creating combined portal session
DEBUG ashpd: Opening portal session
DEBUG ashpd: Waiting for user response...
```

If you see timeout or error here, user needs to click "Share" in dialog.

---

## COMMON ISSUES AND LOG PATTERNS

### Issue: "Portal permission denied"

**Log pattern:**
```
ERROR lamco_portal: Failed to create portal session
ERROR lamco_portal: User denied screen sharing permission
```

**Fix:** User must click "Share" in permission dialog

### Issue: "PipeWire connection failed"

**Log pattern:**
```
ERROR lamco_pipewire: Failed to connect to PipeWire daemon
ERROR lamco_pipewire: Is pipewire.service running?
```

**Fix:** `systemctl --user start pipewire`

### Issue: "No frames captured"

**Log pattern:**
```
INFO lamco_rdp_server::server::display_handler: Display pipeline heartbeat: 1000 iterations, sent 0, dropped 0
WARN lamco_rdp_server::server::display_handler: No frames available
```

**Debug:**
```bash
RUST_LOG=lamco_pipewire=trace,lamco_rdp_server::server::display_handler=trace
```

### Issue: "Input not working"

**Log pattern:**
```
ERROR lamco_portal::remote_desktop: notify_keyboard_keycode failed
ERROR ashpd: D-Bus call failed: Permission denied
```

**Fix:** Portal session needs RemoteDesktop permission, not just ScreenCast

---

## PERFORMANCE IMPACT

### Log Level Performance

| Level | Overhead | Use Case |
|-------|----------|----------|
| error | <0.1% | Production |
| warn | <0.5% | Production |
| info | ~1-2% | Testing |
| debug | ~5-10% | Development |
| trace | ~20-30% | Debugging only |

**Recommendation:** Use `info` for normal testing, `debug` when investigating, `trace` only for specific bugs

### Log Format Performance

| Format | Overhead | Use Case |
|--------|----------|----------|
| compact | Lowest | Production |
| pretty | Low | Development |
| json | Medium | Log aggregation |

---

## ENVIRONMENT VARIABLES

### RUST_LOG Syntax

```bash
# Single target
RUST_LOG=lamco_portal=debug

# Multiple targets, same level
RUST_LOG=lamco_portal=debug,lamco_pipewire=debug

# Multiple targets, different levels
RUST_LOG=lamco_portal=trace,lamco_pipewire=debug,warn

# Wildcard (all lamco crates)
RUST_LOG=lamco=debug

# Override specific module
RUST_LOG=lamco=debug,lamco_portal::clipboard=trace
```

### RUST_BACKTRACE

```bash
# Enable backtraces on panic
RUST_BACKTRACE=1 ./lamco-rdp-server

# Full backtraces (includes all frames)
RUST_BACKTRACE=full ./lamco-rdp-server
```

### Combined

```bash
RUST_LOG=trace RUST_BACKTRACE=1 ./lamco-rdp-server -c config.toml -vvv 2>&1 | tee full-debug.log
```

---

## UPDATING DEFAULT FILTER (TODO)

### Current Problem

`src/main.rs:98-101` has hardcoded filter that doesn't include lamco-* crates:

```rust
tracing_subscriber::EnvFilter::new(format!(
    "lamco_rdp_server={},ironrdp_cliprdr=trace,ironrdp_server=trace,warn",
    log_level
))
```

### Recommended Fix

```rust
tracing_subscriber::EnvFilter::new(format!(
    "lamco={},ironrdp=debug,ashpd=info,warn",
    log_level
))
```

**Why:**
- `lamco={}` enables ALL lamco-* crates
- `ironrdp=debug` shows RDP protocol without spam
- `ashpd=info` shows Portal API calls
- `warn` catches everything else

### Alternative: More Granular

```rust
tracing_subscriber::EnvFilter::new(format!(
    "lamco_rdp_server={},lamco_portal=info,lamco_pipewire=info,\
     lamco_video=info,lamco_rdp_input=info,lamco_clipboard_core=info,\
     lamco_rdp_clipboard=info,ironrdp=debug,ashpd=info,warn",
    log_level
))
```

---

## SUGGESTED IMPROVEMENT: Add Diagnostic Flag

**Proposal:** Add `--diagnose` flag for automatic comprehensive logging

```rust
#[arg(long)]
pub diagnose: bool,

// In init_logging():
let filter = if args.diagnose {
    "trace,h2=warn,hyper=warn"  // Everything except HTTP noise
} else {
    // Current logic
};
```

**Usage:**
```bash
./lamco-rdp-server -c config.toml --diagnose --log-file diagnostic.log
```

---

## TESTING CHECKLIST

### Before Starting Test

- [ ] Know VM IP address
- [ ] Have RDP client ready (Remmina/xfreerdp/Windows RDP)
- [ ] Decide log level (recommend: `RUST_LOG=lamco=info,ironrdp=info,ashpd=debug,warn`)
- [ ] Prepare log file path
- [ ] Have terminal ready to view logs

### During Test

- [ ] Watch for "Server is ready and listening" message
- [ ] Watch for Portal permission dialog (grant it!)
- [ ] Connect from RDP client
- [ ] Watch for "Client connected successfully"
- [ ] Test video (move windows)
- [ ] Test input (type, click)
- [ ] Test clipboard (copy/paste)

### After Test

- [ ] Save log file
- [ ] Note any errors
- [ ] Check for crash/panic
- [ ] Review performance (frame rate, latency)

---

## QUICK REFERENCE

### Minimal Testing
```bash
./lamco-rdp-server -c config.toml
```

### Standard Testing
```bash
RUST_LOG=lamco=info,ironrdp=info ./lamco-rdp-server -c config.toml -v
```

### Debug Mode
```bash
RUST_LOG=lamco=debug,ironrdp=debug ./lamco-rdp-server -c config.toml -vv
```

### Full Diagnostic
```bash
RUST_LOG=trace RUST_BACKTRACE=1 ./lamco-rdp-server -c config.toml -vvv --log-file debug.log 2>&1 | tee console.log
```

---

**READY FOR KDE VM TESTING**

Use Standard Testing command first. Escalate to Debug Mode if issues arise.

**Sources:**
- [Tracing Rust Guide](https://generalistprogrammer.com/tutorials/tracing-log-rust-crate-guide)
- [IronRDP GitHub](https://github.com/Devolutions/IronRDP)
- [Logging in Rust 2025](https://www.shuttle.dev/blog/2023/09/20/logging-in-rust)
