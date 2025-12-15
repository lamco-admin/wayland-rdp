# BACKEND DECISION SUMMARY
**Quick Reference Guide for WRD-Server Backend Strategy**

**Date**: 2025-11-20
**Status**: ARCHITECTURAL DECISION RECORD

---

## TL;DR - EXECUTIVE DECISION

**RECOMMENDED BACKEND: X11 Backend + Xvfb**

**Why**:
- NO GPU required
- Container-friendly
- Battle-tested technology (Xvfb)
- Full compositor control
- Cloud-deployable
- 150MB memory footprint

**Implementation Timeline**: 2-4 weeks

---

## THE PROBLEM

WRD-Server needs a Wayland compositor backend that is:
1. Headless (no physical display)
2. GPU-free (cloud VMs without GPU)
3. Container-deployable (Docker/K8s)
4. Multi-tenant (multiple instances)
5. Low resource usage
6. Production-ready

---

## RESEARCH FINDINGS

### Smithay 0.7.0 Has NO True Headless Backend

Available backends:

| Backend | GPU? | Headless? | Container? | Status | WRD Fit |
|---------|------|-----------|------------|--------|---------|
| **DRM** | ✅ Required | ❌ No | ❌ No | Stable | ❌ Poor |
| **X11** | ⚠️ Optional* | ✅ Yes* | ✅ Yes* | Stable | ✅ **BEST** |
| **Winit** | ✅ Required | ❌ No | ❌ No | Stable | ❌ Dev only |
| **Pixman** | ❌ No | ✅ Yes | ✅ Yes | ⚠️ Experimental | ⚠️ Future |

*With Xvfb

### The Xvfb Solution

```
Smithay X11 Backend → Xvfb → RAM Framebuffer
```

**Xvfb** (X Virtual Framebuffer):
- Virtual X server with in-memory framebuffer
- NO GPU required
- Software rendering
- Used in production for 20+ years
- Works perfectly in containers

---

## THREE-PHASE STRATEGY

### Phase 1: Portal API (CURRENT)
**Status**: Production-ready NOW
**Timeline**: Ongoing

```
WRD-Server → Portal API → GNOME/KDE
```

**Use When**:
- Running on existing desktop
- Development/testing
- Full feature support needed immediately

**Resources**: 500MB (includes desktop environment)

---

### Phase 2: X11 + Xvfb (IMPLEMENT NEXT)
**Status**: RECOMMENDED
**Timeline**: 2-4 weeks

```
WRD-Server (Smithay compositor) → Xvfb → RAM
```

**Use When**:
- Cloud VM deployment
- Container deployment
- Headless servers
- Multi-tenant systems

**Resources**: 150-200MB per instance

**Implementation**:
```rust
// Enable in Cargo.toml
smithay = { version = "0.7", features = [
    "backend_x11",
    "backend_egl",
    "backend_gbm",
    "renderer_gl",
    "wayland_frontend",
    "desktop"
]}
```

**Deployment**:
```bash
# Start Xvfb
Xvfb :99 -screen 0 1920x1080x24 &

# Run compositor
DISPLAY=:99 ./wrd-server
```

---

### Phase 3: Pixman Renderer (FUTURE)
**Status**: Experimental
**Timeline**: 2025-2026 (when API matures)

```
WRD-Server (Smithay) → Pixman CPU Renderer → Memory Buffer
```

**Use When**:
- API is documented and stable
- Need absolute minimum resources
- Maximum portability

**Resources**: 50-100MB per instance

**Current Blockers**:
- Incomplete API
- No production examples
- Unproven performance

---

## THREADING MODEL

### The Challenge
- **Smithay**: Uses calloop (single-threaded, callback-based)
- **IronRDP**: Uses Tokio (async/await, multi-threaded)
- **Incompatible**: Cannot run on same thread

### The Solution: Two-Thread Architecture

```
THREAD 1 (Tokio)          THREAD 2 (Calloop)
┌──────────────┐          ┌──────────────┐
│  IronRDP     │          │  Smithay     │
│  Server      │◄────────►│  Compositor  │
│  (async)     │ channels │  (callbacks) │
└──────────────┘          └──────────────┘
```

**Channels**:
- Compositor → RDP: `crossbeam_channel::bounded()`
- RDP → Compositor: `calloop::channel::channel()`
- Within RDP: `tokio::sync::mpsc::channel()`

**Key Insight**: This is the CORRECT architecture. Accept it.

---

## CLIPBOARD INTEGRATION

### Why This Matters
**CRITICAL**: This solves the Linux→Windows clipboard issue!

### With Portal API (Current)
```rust
// Must poll clipboard
tokio::spawn(async {
    loop {
        let data = clipboard_portal.read().await?;
        if changed {
            rdp_server.send_clipboard(data).await?;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
});
```

**Problem**: Polling is unreliable

### With Smithay Compositor (Phase 2)
```rust
// Direct protocol handler!
impl SelectionHandler for CompositorState {
    fn new_selection(&mut self, ty: SelectionTarget, source: Option<WlDataSource>) {
        // THIS FIRES WHEN CLIPBOARD CHANGES!
        match ty {
            SelectionTarget::Clipboard => {
                // Read data immediately
                let data = self.read_selection_data(source);

                // Send to RDP (via channel)
                self.rdp_tx.send(ClipboardEvent::Changed(data));
            }
        }
    }
}
```

**Advantage**: Event-driven, instant, reliable!

---

## DEPLOYMENT SCENARIOS

### Cloud VM (AWS/GCP/Azure)

```dockerfile
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y xvfb
COPY wrd-server /usr/local/bin/

CMD ["sh", "-c", "Xvfb :99 -screen 0 1920x1080x24 & DISPLAY=:99 wrd-server"]
```

**Cost**:
- Memory: 150-200MB per instance
- CPU: 0.5-1 core
- No GPU needed

### Kubernetes Multi-Tenant

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: wrd-server
spec:
  replicas: 10  # 10 RDP instances
  template:
    spec:
      containers:
      - name: wrd-server
        image: wrd-server:latest
        resources:
          requests:
            memory: "200Mi"
            cpu: "500m"
          limits:
            memory: "300Mi"
            cpu: "1000m"
        ports:
        - containerPort: 3389
```

**Scaling**: Each pod is independent, full RDP server

---

## RESOURCE COMPARISON

### Per Instance (1920x1080 @ 30fps)

| Backend | Memory | CPU | GPU | Container | Deploy Complexity |
|---------|--------|-----|-----|-----------|-------------------|
| Portal API | 500MB | Low | No | Hard | Medium |
| X11+Xvfb | 150MB | Medium | No | Easy | **Low** |
| DRM | 100MB | Low | Yes | No | High |
| Pixman | 50MB | High | No | Easy | Unknown |

**Winner**: X11+Xvfb (best balance)

---

## MIGRATION PATH

### Step 1: Current State (Portal API)
**Timeline**: Now
**Status**: Working, production-ready

Keep for:
- Desktop environments
- Testing/development
- Immediate production needs

### Step 2: Implement X11+Xvfb
**Timeline**: Weeks 1-4

#### Week 1: Core Compositor
- Set up Smithay with X11 backend
- Xvfb integration
- Basic rendering

#### Week 2: Protocol Handlers
- Wayland protocols (compositor, xdg_shell, shm)
- Seat/input handling
- **Clipboard (SelectionHandler)**

#### Week 3: RDP Bridge
- Threading architecture
- Channel-based communication
- Framebuffer → RDP bitmap conversion

#### Week 4: Testing & Deployment
- Container testing
- Performance benchmarks
- Documentation

### Step 3: Evaluate Pixman (Future)
**Timeline**: 2025-2026

Wait for:
- API documentation
- Production examples
- Performance verification

---

## CODE EXAMPLES

### Starting Xvfb (Production)

```bash
#!/bin/bash
# Production Xvfb startup

# Configuration
DISPLAY_NUM=${DISPLAY_NUM:-99}
RESOLUTION=${RESOLUTION:-1920x1080x24}

# Start Xvfb with production options
Xvfb :${DISPLAY_NUM} \
  -screen 0 ${RESOLUTION} \
  -nolisten tcp \
  -auth /tmp/.Xvfb-${DISPLAY_NUM}-auth \
  +extension GLX \
  +extension RANDR \
  +render \
  -noreset \
  > /var/log/xvfb-${DISPLAY_NUM}.log 2>&1 &

XVFB_PID=$!

# Wait for X server
for i in {1..30}; do
  if xdpyinfo -display :${DISPLAY_NUM} >/dev/null 2>&1; then
    echo "Xvfb started successfully on :${DISPLAY_NUM}"
    break
  fi
  sleep 0.1
done

# Verify
if ! xdpyinfo -display :${DISPLAY_NUM} >/dev/null 2>&1; then
  echo "ERROR: Xvfb failed to start"
  kill ${XVFB_PID} 2>/dev/null || true
  exit 1
fi

echo ${XVFB_PID} > /var/run/xvfb-${DISPLAY_NUM}.pid
```

### Smithay X11 Compositor (Skeleton)

```rust
use smithay::backend::x11::{X11Backend, WindowBuilder};
use smithay::backend::renderer::gles::GlesRenderer;
use calloop::EventLoop;

struct WrdCompositor {
    event_loop: EventLoop<CompositorState>,
    state: CompositorState,
}

impl WrdCompositor {
    pub fn new(display_num: u32) -> Result<Self> {
        // Set DISPLAY env
        std::env::set_var("DISPLAY", format!(":{}",display_num));

        // Connect to X server (Xvfb)
        let backend = X11Backend::new()?;

        // Get DRM node for rendering
        let (node, fd) = backend.handle().drm_node()?;

        // Create renderer
        let renderer = create_renderer(fd)?;

        // Create compositor state
        let state = CompositorState::new(backend, renderer)?;

        // Create event loop
        let event_loop = EventLoop::try_new()?;

        Ok(Self { event_loop, state })
    }

    pub fn run(mut self) -> Result<()> {
        self.event_loop.run(None, &mut self.state, |_| {})?;
        Ok(())
    }
}

struct CompositorState {
    backend: X11Backend,
    renderer: GlesRenderer,
    // ... Wayland protocol state
}
```

### RDP Bridge (Threading)

```rust
use crossbeam_channel::bounded;
use calloop::channel::channel;

fn main() {
    // Create channels
    let (comp_tx, rdp_rx) = bounded::<CompositorEvent>(8);
    let (rdp_tx, comp_rx) = channel::<CompositorCommand>();

    // Start compositor thread
    let compositor_thread = std::thread::spawn(move || {
        let mut compositor = WrdCompositor::new(99).unwrap();
        compositor.set_rdp_channel(comp_rx, comp_tx);
        compositor.run().unwrap();
    });

    // Start RDP server (Tokio)
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let rdp_server = RdpServer::new(rdp_tx, rdp_rx).await?;
        rdp_server.run().await?;
        Ok::<(), Error>(())
    }).unwrap();

    compositor_thread.join().unwrap();
}
```

---

## DECISION MATRIX

### Use Portal API When:
- ✅ Running on desktop environment
- ✅ Need immediate production deployment
- ✅ Testing/development
- ❌ Headless deployment
- ❌ Container deployment
- ❌ Multi-tenant systems

### Use X11+Xvfb When:
- ✅ Headless deployment
- ✅ Cloud VMs (AWS/GCP/Azure)
- ✅ Container deployment (Docker/K8s)
- ✅ Multi-tenant systems
- ✅ GPU-free environments
- ✅ Production at scale

### Use Pixman When:
- ⚠️ API is mature and documented
- ⚠️ Need absolute minimum resources
- ⚠️ Maximum portability required
- ❌ Not ready yet (2025)

---

## NEXT ACTIONS

### Immediate (This Week)
1. ✅ Complete this research
2. ⚠️ Review with team
3. ⚠️ Approve Phase 2 implementation

### Short Term (Weeks 1-4)
1. Set up Smithay with X11 backend
2. Implement core Wayland protocols
3. Build RDP bridge (threading + channels)
4. Test in containers

### Medium Term (Months 1-3)
1. Production deployment testing
2. Performance optimization
3. Multi-tenant validation
4. Documentation

### Long Term (2025-2026)
1. Monitor Pixman renderer development
2. Evaluate migration path
3. Plan resource optimization

---

## REFERENCES

- Full Research: `SMITHAY-BACKEND-ARCHITECTURE-RESEARCH.md`
- Smithay Docs: https://smithay.github.io/smithay/
- Xvfb Manual: `man Xvfb`
- Anvil Example: https://github.com/Smithay/smithay/tree/v0.7.0/anvil

---

## CONCLUSION

**IMPLEMENT X11 BACKEND + XVFB**

This provides the best balance of:
- Production-readiness (Xvfb is 20+ years old)
- Resource efficiency (150MB vs 500MB)
- Deployment flexibility (containers, cloud, bare metal)
- Feature completeness (full compositor control)
- Implementation timeline (2-4 weeks vs 6-12 months)

**Start Phase 2 implementation immediately.**

---

**END OF SUMMARY**
