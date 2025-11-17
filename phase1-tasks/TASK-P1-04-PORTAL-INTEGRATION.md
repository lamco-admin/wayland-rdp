# TASK P1-04: PORTAL INTEGRATION
**Task ID:** TASK-P1-04
**Phase:** 1  
**Milestone:** Portal Integration
**Duration:** 7-10 days
**Dependencies:** TASK-P1-01 (Foundation)
**Status:** NOT_STARTED

## OBJECTIVE
Implement complete xdg-desktop-portal integration using ashpd for ScreenCast, RemoteDesktop, and Clipboard portals.

## SUCCESS CRITERIA
- ✅ D-Bus session connection established
- ✅ Portal session creates successfully
- ✅ User permission dialog appears
- ✅ PipeWire file descriptor obtained
- ✅ Multiple monitors detected
- ✅ Input injection via RemoteDesktop portal works
- ✅ Clipboard portal accessible

## IMPLEMENTATION

### 1. Portal Manager (`src/portal/mod.rs`)
```rust
use ashpd::desktop::{screencast::ScreenCast, remote_desktop::RemoteDesktop};
use ashpd::zbus;
use std::sync::Arc;

pub struct PortalManager {
    connection: zbus::Connection,
    screencast: Arc<ScreenCastManager>,
    remote_desktop: Arc<RemoteDesktopManager>,
    clipboard: Arc<ClipboardManager>,
}

impl PortalManager {
    pub async fn new(config: &Arc<Config>) -> Result<Self> {
        let connection = zbus::Connection::session().await?;
        
        let screencast = Arc::new(ScreenCastManager::new(connection.clone(), config.clone()).await?);
        let remote_desktop = Arc::new(RemoteDesktopManager::new(connection.clone(), config.clone()).await?);
        let clipboard = Arc::new(ClipboardManager::new(connection.clone(), config.clone()).await?);
        
        Ok(Self { connection, screencast, remote_desktop, clipboard })
    }
    
    pub async fn create_session(&self) -> Result<PortalSessionHandle> {
        let rd_session = self.remote_desktop.create_session().await?;
        let devices = DeviceType::Keyboard | DeviceType::Pointer;
        self.remote_desktop.select_devices(&rd_session, devices).await?;
        
        let (pipewire_fd, streams) = self.remote_desktop.start_session(&rd_session).await?;
        
        Ok(PortalSessionHandle::new(rd_session, pipewire_fd, streams))
    }
}
```

### 2. ScreenCast Manager (`src/portal/screencast.rs`)
```rust
use ashpd::desktop::screencast::*;

pub struct ScreenCastManager {
    connection: zbus::Connection,
    config: Arc<Config>,
}

impl ScreenCastManager {
    pub async fn create_session(&self) -> Result<ScreenCastSession> {
        let proxy = ScreenCast::new(&self.connection).await?;
        let session = proxy.create_session().await?;
        Ok(session)
    }
    
    pub async fn select_sources(&self, session: &ScreenCastSession, source_types: SourceType) -> Result<()> {
        let proxy = ScreenCast::new(&self.connection).await?;
        proxy.select_sources(session, source_types, true, self.config.video.cursor_mode.parse()?).await?;
        Ok(())
    }
    
    pub async fn start(&self, session: &ScreenCastSession) -> Result<(RawFd, Vec<Stream>)> {
        let proxy = ScreenCast::new(&self.connection).await?;
        let response = proxy.start(session).await?;
        Ok((response.fd, response.streams))
    }
}
```

### 3. RemoteDesktop Manager (`src/portal/remote_desktop.rs`)
```rust
use ashpd::desktop::remote_desktop::*;

pub struct RemoteDesktopManager {
    connection: zbus::Connection,
    config: Arc<Config>,
}

impl RemoteDesktopManager {
    pub async fn create_session(&self) -> Result<RemoteDesktopSession> {
        let proxy = RemoteDesktop::new(&self.connection).await?;
        Ok(proxy.create_session().await?)
    }
    
    pub async fn select_devices(&self, session: &RemoteDesktopSession, devices: DeviceType) -> Result<()> {
        let proxy = RemoteDesktop::new(&self.connection).await?;
        proxy.select_devices(session, devices, None).await?;
        Ok(())
    }
    
    pub async fn start_session(&self, session: &RemoteDesktopSession) -> Result<(RawFd, Vec<Stream>)> {
        let proxy = RemoteDesktop::new(&self.connection).await?;
        let response = proxy.start(session).await?;
        Ok((response.fd, response.streams))
    }
    
    pub async fn notify_pointer_motion(&self, session: &RemoteDesktopSession, dx: f64, dy: f64) -> Result<()> {
        let proxy = RemoteDesktop::new(&self.connection).await?;
        proxy.notify_pointer_motion(session, dx, dy).await?;
        Ok(())
    }
    
    pub async fn notify_keyboard_keycode(&self, session: &RemoteDesktopSession, keycode: i32, state: KeyState) -> Result<()> {
        let proxy = RemoteDesktop::new(&self.connection).await?;
        proxy.notify_keyboard_keycode(session, keycode, state).await?;
        Ok(())
    }
}
```

### 4. Session Handle (`src/portal/session.rs`)
```rust
pub struct PortalSessionHandle {
    session_id: String,
    pipewire_fd: RawFd,
    streams: Vec<StreamInfo>,
}

impl PortalSessionHandle {
    pub fn new(session_id: String, pipewire_fd: RawFd, streams: Vec<StreamInfo>) -> Self {
        Self { session_id, pipewire_fd, streams }
    }
    
    pub fn pipewire_fd(&self) -> RawFd {
        self.pipewire_fd
    }
    
    pub fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }
    
    pub async fn close(self) -> Result<()> {
        // Close PipeWire FD
        unsafe { libc::close(self.pipewire_fd); }
        Ok(())
    }
}
```

## VERIFICATION
- [ ] Portal session creates without errors
- [ ] Permission dialog appears on desktop
- [ ] User can approve/deny access
- [ ] PipeWire FD is valid
- [ ] Stream metadata correct
- [ ] Input injection methods callable
- [ ] Clipboard accessible

## INTEGRATION TEST
```rust
#[tokio::test]
async fn test_portal_session_creation() {
    let config = Config::default_config().unwrap();
    let portal_manager = PortalManager::new(&Arc::new(config)).await.unwrap();
    let session = portal_manager.create_session().await.unwrap();
    assert!(session.pipewire_fd() > 0);
    assert!(!session.streams().is_empty());
}
```

## COMPLETION CRITERIA
- All portal APIs accessible
- Session creation working
- PipeWire FD obtained
- Input methods functional
- Tests passing

**Time:** 7-10 days
