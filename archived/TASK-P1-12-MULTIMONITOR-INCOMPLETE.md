# TASK P1-12: MULTI-MONITOR SUPPORT
**Task ID:** TASK-P1-12
**Duration:** 5-7 days
**Dependencies:** TASK-P1-05, P1-08, P1-09
**Status:** NOT_STARTED

## OBJECTIVE
Implement multi-monitor support for up to 4 displays.

## SUCCESS CRITERIA
- ✅ Multiple monitors detected
- ✅ Each monitor streams independently
- ✅ Layout calculated correctly
- ✅ RDP client shows all monitors
- ✅ Input coordinates map correctly
- ✅ Can move windows between monitors

## KEY MODULES
- `src/multimon/manager.rs` - Monitor manager
- `src/multimon/layout.rs` - Layout calculator

## CORE IMPLEMENTATION
```rust
pub struct MultiMonitorManager {
    monitors: Vec<MonitorInfo>,
    streams: Vec<PipeWireStream>,
    pipelines: Vec<VideoPipeline>,
}

pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub primary: bool,
}

impl MultiMonitorManager {
    pub async fn new(portal: &PortalManager) -> Result<Self>;
    pub async fn run(&mut self) -> Result<()>;
}
```

## DELIVERABLES
1. Monitor detection
2. Layout calculation
3. Per-monitor streams
4. Coordinate mapping
5. RDP topology configuration
6. Multi-monitor tests

**Time:** 5-7 days
