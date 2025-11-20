# VM Setup Commands - Run These Manually

SSH to VM and run these commands (will prompt for password):

```bash
cd ~/wayland-rdp

# Install dependencies
sudo apt-get update
sudo apt-get install -y xvfb libxkbcommon-dev libgbm-dev libegl1-mesa-dev libgl1-mesa-dev libwayland-dev

# Build compositor mode
~/.cargo/bin/cargo build --release --features headless-compositor

# Should succeed now!
```

**After build succeeds, test**:
```bash
# Start Xvfb (virtual display)
Xvfb :99 -screen 0 1920x1080x24 &

# Run compositor mode
DISPLAY=:99 ./target/release/wrd-server --mode compositor -c config.toml --log-file comp-test.log -vv
```

---

## THE DECISION YOU NEED TO MAKE

**Option 1: Dual Mode (Recommended)**
- Portal for desktop screen sharing (current, works great)
- Compositor for VDI/cloud (new, full clipboard)
- Support both use cases

**Option 2: Compositor Only**
- Always use Lamco
- Works everywhere
- Full clipboard always
- But no "screen sharing" of existing desktops

**Option 3: Portal Only**
- Simpler codebase
- Works on all desktops
- Accept clipboard limitation (Windows→Linux only)

**My vote: Option 1 (Dual Mode)**
- Portal mode works NOW for 80% of users
- Compositor mode for 20% who need full clipboard
- Best of both worlds

What's your call?
