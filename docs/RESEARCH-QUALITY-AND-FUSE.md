# Research: Video Quality Optimization & FUSE in Flatpak

**Date**: 2026-01-15
**Purpose**: Deep research into damage detection, artifact prevention, adaptive quality, and FUSE sandboxing for VDI excellence.

---

## Table of Contents

1. [Damage Detection Strategies](#1-damage-detection-strategies)
2. [Artifact Recovery Techniques](#2-artifact-recovery-techniques)
3. [Adaptive Quality / Auto-Tuning](#3-adaptive-quality--auto-tuning)
4. [FUSE in Flatpak Sandboxes](#4-fuse-in-flatpak-sandboxes)
5. [Recommendations for lamco-rdp-server](#5-recommendations-for-lamco-rdp-server)

---

## 1. Damage Detection Strategies

### 1.1 X11 XDamage Extension

The [XDamage Extension](https://www.x.org/releases/current/doc/damageproto/damageproto.txt) is the gold standard for X11 damage tracking:

- **How it works**: Applications register for damage notifications on drawables. The X server monitors all rendering operations and reports changed regions.
- **Event structure**: `XDamageNotifyEvent` contains the damaged rectangle (`area`), drawable ID, and timestamp.
- **Optimization**: Can receive raw rectangles as events, or have them partially processed within the X server to reduce data volume.

**Key insight**: FreeRDP's shadow server uses XDamage for efficient change detection, only capturing regions that actually changed.

### 1.2 FreeRDP Shadow Server Approach

From the [FreeRDP DeepWiki documentation](https://deepwiki.com/FreeRDP/FreeRDP/4.1-shadow-server):

```c
// shadow_capture_compare_with_format() implements:
// - 16x16 pixel tile-based comparison
// - Format-aware comparison (supports different pixel formats)
// - Alpha channel handling
// - Returns bounding rectangle of changed tiles
```

**Source files**:
- `server/shadow/X11/x11_shadow.c`
- `server/shadow/shadow_subsystem.c`

FreeRDP uses a **16x16 tile grid** and compares tiles to detect changes. This is similar to our approach but with a fixed tile size.

### 1.3 Wayland/wlroots Damage Tracking

From [emersion's damage tracking article](https://emersion.fr/blog/2019/intro-to-damage-tracking/):

**Levels of damage tracking**:

1. **Level 0**: Stop rendering when nothing changes (frame-wise)
2. **Level 1**: Track which surfaces changed, only re-composite those
3. **Level 2**: Track exact rectangles within surfaces (pixel-perfect)

**EGL_EXT_buffer_age**:
- Tells compositor how old the current back buffer is
- Allows re-using previous frame data outside damaged regions
- Critical for double/triple buffering efficiency

**Implementation in wlroots**:
```c
// wlr_output_damage_attach_render() fills buffer damage region
// wlr_surface_get_effective_damage() gets surface-local frame damage
// Damage accumulates from surface commits
```

**Swiss cheese problem**: Many small damage rectangles can hurt GPU performance. Solution: simplify damage by computing region extents if rectangle count is too high.

### 1.4 VNC Dirty Rectangle Detection

From [TightVNC documentation](https://www.tightvnc.com/archive/compare.html):

- VNC servers compare frame buffers to detect changed regions
- **TightVNC encoding** splits rectangles based on color complexity (solid areas vs. complex images)
- Lower compression levels favor smaller subrectangles for better per-region optimization
- **TigerVNC** uses libjpeg-turbo for accelerated Tight encoding with SIMD

### 1.5 Spice Protocol (QEMU)

From [Spice documentation](https://www.spice-space.org/spice-user-manual.html):

- QXL driver translates OS commands to QXL commands pushed to command ring
- libspice uses a **graphics commands tree** to:
  - Drop commands hidden by other commands
  - Detect video streams
  - Optimize transmission order

---

## 2. Artifact Recovery Techniques

### 2.1 IDR Frames (Instantaneous Decoder Refresh)

From [Streaming Learning Center](https://streaminglearningcenter.com/encoding/everything-you-ever-wanted-to-know-about-idr-frames-but-were-afraid-to-ask.html):

**Key facts**:
- Every IDR frame is an I-frame, but not vice versa
- IDR clears the decoder's reference picture buffer
- **Eliminates error propagation** - artifacts accumulated in reference frames are discarded
- Critical for: random access, stream switching, error recovery

**Best practice for streaming**: Every I-frame should be an IDR frame.

**Recommended intervals**:
| Use Case | IDR Interval |
|----------|-------------|
| Broadcast TV | 1-2 seconds |
| DVD | ~0.5 seconds |
| Blu-ray | 1 second |
| Internet streaming | 2-10 seconds |
| Low-latency gaming | 1-2 seconds |

### 2.2 Periodic Intra Refresh (PIR)

From [x264 documentation](https://x264-devel.videolan.narkive.com/etMiOxTb/how-to-enable-intra-refresh):

**Alternative to IDR frames**:
- No full I-frames after the first
- Columns/rows of macroblocks coded in Intra mode every frame
- Creates a **"refresh wave"** that sweeps across the image
- Entire frame refreshed over `--keyint` frames

**Benefits**:
- More constant frame sizes (better for low-latency VBV)
- Increased resilience to packet loss
- Smoother bandwidth usage

**Drawbacks**:
- Visible refresh column at low bitrates
- Reduced compression efficiency
- **Not supported by OpenH264** (only x264)
- Compatibility issues with open-GOP

### 2.3 Periodic Full Frame Strategy (Your Idea!)

**Concept**: Force a full IDR keyframe every N seconds regardless of scene changes.

**How other systems handle this**:

| System | Approach |
|--------|----------|
| WebRTC | Sends IDR on Picture Loss Indication (PLI) from client |
| Broadcast | Fixed keyframe interval (1-2 sec) |
| Microsoft RDP | Scene change detection + configurable max interval |
| Citrix HDX | Adaptive based on network conditions |

**Implementation options**:
1. **Fixed interval**: Force IDR every N seconds (e.g., 5-10 sec)
2. **Adaptive interval**: Shorter interval when artifacts detected, longer when stable
3. **Client-requested**: Client sends PLI when it detects visual issues
4. **Hybrid**: Fixed max interval + scene change detection + client PLI

### 2.4 Scene Change Detection

**How it works**:
- Measure inter-frame difference (histogram comparison, edge detection, motion vectors)
- If difference exceeds threshold, insert IDR
- Prevents compression artifacts during scene transitions

**x264 implementation**: `--scenecut` option (default: 40)

---

## 3. Adaptive Quality / Auto-Tuning

### 3.1 Microsoft RDP Dynamic Network Detection

From [Microsoft Learn - Graphics Encoding](https://learn.microsoft.com/en-us/azure/virtual-desktop/graphics-encoding):

**Key capabilities**:
- **Continuous network detection**: Monitors bandwidth and RTT in real-time
- **Dynamic codec selection**: Switches between codecs based on conditions
- **Adaptive graphics**: Adjusts encoding quality based on bandwidth and content
- **Delta detection + caching**: Reduces transmitted data

**Behavior**: "RDP uses the full network pipe when available and rapidly backs off when the network is needed for something else."

### 3.2 Citrix HDX Adaptive Display

From [Citrix HDX documentation](https://docs.citrix.com/en-us/citrix-virtual-apps-desktops/technical-overview/hdx.html):

**Three core principles**:

1. **Intelligent Redirection**: Examines screen activity, application commands, device capabilities to decide where to render (client vs. server)

2. **Adaptive Compression**:
   - Evaluates input type (text, video, voice, multimedia)
   - Chooses optimal codec and CPU/GPU proportion
   - Adapts based on each unique user

3. **Data De-duplication**: Caches repeated patterns to eliminate duplicate traffic

**Adaptive Display** (successor to Progressive Display):
- **Zero configuration** - auto-adapts to bandwidth changes
- Eliminates need for complex policy configurations
- "Provides fantastic out-of-the-box experience"

**Thinwire**: Reduces bandwidth while Adaptive Display auto-adjusts quality.

### 3.3 VMware Blast Protocol

From [comparative whitepaper](https://www.theitvortex.com/whitepaper-a-comparative-study-of-pcoip-and-blast-protocols-for-horizon-view/):

**Encoder switch** can dynamically switch between:
- Blast Codec (proprietary adaptive)
- JPG/PNG (static images)
- H.264 codec

Decision based on content type and administrator policy.

### 3.4 Machine Learning ABR Algorithms

From [MIT CSAIL Pensieve](https://www.csail.mit.edu/research/neural-adaptive-bitrate-streaming-using-reinforcement-learning):

**Pensieve**: Uses Reinforcement Learning to train neural network for bitrate selection:
- Observes: buffer level, throughput history, video chunk sizes
- Learns: optimal policy without fixed heuristics
- Adapts: to diverse network conditions

**Oboe**: Auto-tuning video ABR algorithms to network conditions (ACM SIGCOMM).

**Meta-RL approaches**: Rapidly adapt control policy to changing network dynamics using probabilistic latent encoder.

### 3.5 Frame Rate vs. Quality Tradeoffs

From [QoE research](https://www.fastpix.io/blog/video-streaming-quality-the-role-of-bitrate-and-resolution):

| Content Type | Priority |
|--------------|----------|
| Sports/Gaming | Frame rate (720p60 > 1080p30) |
| Presentations | Resolution (text clarity) |
| Movies | Balance (quality > smoothness) |
| VDI/Remote Desktop | **Latency > all** |

**VDI-specific considerations**:
- Mouse cursor responsiveness is critical
- Text must remain sharp
- Window movement needs smooth rendering
- Typing latency must be imperceptible

---

## 4. FUSE in Flatpak Sandboxes

### 4.1 Why FUSE Doesn't Work in Flatpak

**Root cause**: Flatpak sandboxes don't allow arbitrary FUSE mounts.

From [Flatpak documentation](https://docs.flatpak.org/en/latest/sandbox-permissions.html):

- Flatpak uses FUSE internally for the **Document Portal** (`/run/flatpak/doc`)
- Apps can't create their own FUSE mounts because:
  - No access to `/dev/fuse`
  - `fusermount3` runs outside the sandbox
  - AppArmor/SELinux restrictions

**Error we see**: "Failed to mount FUSE: No such file or directory" (ENOENT for fusermount3)

### 4.2 AppArmor/fusermount3 Issues

From [Ubuntu bug #2100295](https://bugs.launchpad.net/ubuntu/+source/apparmor/+bug/2100295):

- fusermount3 requires access to `/run/mount/utab` and related files
- AppArmor blocks this access in confined contexts
- Flatpak runs fusermount3 in a mount namespace with limited fd access

**Fix in Ubuntu 25.10**: Loosened AppArmor confinement, but this only helps Flatpak's internal FUSE usage, not app-created mounts.

### 4.3 Possible Workarounds

#### Option A: flatpak-spawn Escape (Hacky)

From [GitHub discussions](https://github.com/flatpak/flatpak/issues/5321):

```bash
# Create fake fusermount in /app/bin that calls host fusermount
#!/bin/sh
exec flatpak-spawn --host fusermount3 "$@"
```

**Downsides**: Breaks sandbox isolation, requires `--talk-name=org.freedesktop.Flatpak`

#### Option B: Document Portal (Limited)

- Use XDG Document Portal for file access
- Files available at `/run/flatpak/doc/`
- **Limitation**: Only for file chooser-selected files, not arbitrary mounts

#### Option C: Host FUSE Mount (System Admin)

- Mount FUSE filesystem on host, expose to Flatpak via `--filesystem`
- Requires out-of-sandbox helper process
- **Limitation**: Complex setup, not self-contained

#### Option D: Accept Staging Fallback (Current)

- FUSE mount fails gracefully
- Files downloaded upfront to staging directory
- **Works correctly**, just less efficient

### 4.4 FUSE Configuration for Non-Flatpak

For native/systemd deployments:

**/etc/fuse.conf**:
```ini
# Allow users to use allow_other mount option
user_allow_other

# Maximum mounts per user
mount_max = 1000
```

**fusermount3 permissions**:
```bash
# Must be setuid root or have CAP_SYS_ADMIN
ls -la /usr/bin/fusermount3
# -rwsr-xr-x 1 root root ... /usr/bin/fusermount3
```

---

## 5. Recommendations for lamco-rdp-server

### 5.1 Periodic Keyframe Injection (Your Idea - Implement!)

**Configuration option**:
```toml
[egfx]
# Force IDR keyframe every N seconds to clear artifacts
# 0 = disabled (rely on scene change detection only)
# Recommended: 5-10 seconds for VDI, 2-3 for unreliable networks
periodic_idr_interval = 10

# Also insert IDR when frame difference exceeds threshold
scene_change_threshold = 0.7
```

**Implementation**:
```rust
// In AVC444 encoder
let elapsed = last_idr_time.elapsed();
let force_idr = elapsed >= Duration::from_secs(config.periodic_idr_interval);

if force_idr || scene_change_detected {
    encoder.force_idr();
    last_idr_time = Instant::now();
}
```

### 5.2 Improved Damage Detection

**Current issues**:
- Typing not detected at certain thresholds
- Window movement causes artifacts

**Recommended changes**:

```toml
[damage_tracking]
enabled = true
method = "diff"

# Tile size affects granularity
# Smaller = more precise but more CPU
# Larger = less precise but faster
tile_size = 16  # Reduce from 32 for character-level detection

# Minimum pixels changed per tile to trigger update
# Lower = more sensitive
diff_threshold = 0.01  # 1% of tile pixels (was 0.02)

# Absolute pixel value difference threshold
# Lower = detect subtle changes
pixel_threshold = 1  # Single pixel difference (was 2)

# Minimum changed area to send update
# Prevents noise from triggering updates
min_region_area = 32  # 32 pixels minimum

# Merge nearby damage regions
# Higher = fewer, larger regions (better for encoding)
merge_distance = 8  # Reduce to preserve precision
```

### 5.3 Auto-Tuning Quality (Future Innovation)

**Concept**: Learn optimal parameters from runtime observations.

**Metrics to observe**:
- Network RTT and jitter
- Packet loss rate
- Client frame ack latency
- Encoder output size variance

**Parameters to tune**:
- QP (quality parameter): Lower = better quality, higher bitrate
- Frame rate target
- IDR interval
- Damage detection sensitivity

**Algorithm sketch**:
```rust
struct AdaptiveQuality {
    // Exponential moving averages
    avg_rtt: f64,
    avg_ack_delay: f64,
    avg_frame_size: f64,

    // Targets
    target_latency_ms: u32,
    target_bitrate: u32,
}

impl AdaptiveQuality {
    fn adjust(&mut self, metrics: &FrameMetrics) {
        // If ack delay increasing, reduce quality
        if metrics.ack_delay > self.target_latency_ms * 2 {
            self.increase_qp();  // Lower quality
            self.reduce_frame_rate();
        }

        // If bandwidth available, improve quality
        if self.avg_rtt < 50.0 && self.avg_frame_size < self.target_bitrate * 0.7 {
            self.decrease_qp();  // Higher quality
        }

        // If packet loss detected, increase IDR frequency
        if metrics.packet_loss > 0.01 {
            self.decrease_idr_interval();
        }
    }
}
```

### 5.4 FUSE Recommendations

**For Flatpak**: Accept staging fallback (current behavior is correct).

**For native/systemd deployment**:
1. Document `/etc/fuse.conf` requirement in deployment guide
2. Add systemd unit that sets up FUSE permissions
3. Consider AppImage distribution for fully self-contained deployment

**Documentation to add**:
```markdown
## FUSE Clipboard Requirements (Native Only)

For on-demand file transfer via FUSE virtual filesystem:

1. Ensure /etc/fuse.conf contains:
   ```
   user_allow_other
   ```

2. Verify fusermount3 has correct permissions:
   ```bash
   ls -la /usr/bin/fusermount3
   # Should show setuid bit: -rwsr-xr-x
   ```

3. User must be in 'fuse' group (some distributions):
   ```bash
   sudo usermod -a -G fuse $USER
   ```

Note: Flatpak sandboxes do not support FUSE mounts.
Files will be downloaded upfront (staging mode) in Flatpak.
```

---

## 6. Summary of Key Innovations

| Innovation | Description | Complexity | Impact |
|------------|-------------|------------|--------|
| **Periodic IDR** | Force keyframe every N seconds | Low | High - clears artifacts |
| **Finer damage tiles** | 16x16 instead of 32x32 | Low | Medium - better character detection |
| **Lower thresholds** | 1% diff, 1 pixel threshold | Low | Medium - more sensitive |
| **Auto-tune QP** | Adjust based on network metrics | Medium | High - adaptive quality |
| **Client PLI support** | Client requests IDR on artifacts | Medium | High - on-demand recovery |
| **ML-based ABR** | RL for optimal bitrate selection | High | Very High - innovation |

---

## 7. Sources

### Damage Detection
- [X11 DAMAGE Extension Protocol](https://www.x.org/releases/current/doc/damageproto/damageproto.txt)
- [FreeRDP Graphics & Display - DeepWiki](https://deepwiki.com/FreeRDP/FreeRDP/6-graphics-and-display)
- [Introduction to Damage Tracking - emersion](https://emersion.fr/blog/2019/intro-to-damage-tracking/)
- [wlroots PR #571 - Output Damage Tracking](https://github.com/swaywm/wlroots/pull/571)
- [TightVNC Encoder Comparison](https://www.tightvnc.com/archive/compare.html)

### Artifact Recovery
- [IDR Frames Explained - Streaming Learning Center](https://streaminglearningcenter.com/encoding/everything-you-ever-wanted-to-know-about-idr-frames-but-were-afraid-to-ask.html)
- [x264 Intra Refresh](https://x264-devel.videolan.narkive.com/etMiOxTb/how-to-enable-intra-refresh)
- [OpenH264 PIR Issue #2014](https://github.com/cisco/openh264/issues/2014)

### Adaptive Quality
- [Microsoft RDP Graphics Encoding](https://learn.microsoft.com/en-us/azure/virtual-desktop/graphics-encoding)
- [Citrix HDX Documentation](https://docs.citrix.com/en-us/citrix-virtual-apps-desktops/technical-overview/hdx.html)
- [MIT CSAIL Pensieve](https://www.csail.mit.edu/research/neural-adaptive-bitrate-streaming-using-reinforcement-learning)
- [MS-RDPEGFX Specification](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpegfx/)

### FUSE in Flatpak
- [Flatpak Sandbox Permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html)
- [Ubuntu AppArmor/fusermount3 Bug](https://bugs.launchpad.net/ubuntu/+source/apparmor/+bug/2100295)
- [Flatpak FUSE Issues](https://github.com/flatpak/flatpak/issues/5694)

### QoE Research
- [Video Streaming Quality - FastPix](https://www.fastpix.io/blog/video-streaming-quality-the-role-of-bitrate-and-resolution)
- [Adaptive Bitrate Streaming - Wikipedia](https://en.wikipedia.org/wiki/Adaptive_bitrate_streaming)
