# Testing with Truly Static Screen - Instructions

**Goal**: Eliminate ALL screen changes to test if P-frames work with perfectly static input.

---

## Method 1: Hide Mouse Cursor (Recommended)

### On the Test Server (192.168.10.205)

**Step 1: SSH to server**
```bash
ssh greg@192.168.10.205
```

**Step 2: Hide the mouse cursor**
```bash
# Install unclutter if not already installed
sudo apt-get install unclutter -y

# Start unclutter to hide cursor after 1 second of inactivity
unclutter -idle 1 -root &

# Note the PID so you can kill it later
echo $! > /tmp/unclutter.pid
```

**Step 3: Kill any animations/notifications**
```bash
# Disable notifications
killall -9 notify-osd 2>/dev/null || true
killall -9 dunst 2>/dev/null || true

# Stop any background updates
killall -9 update-notifier 2>/dev/null || true
```

**Step 4: Start the RDP server**
```bash
cd ~
./run-server.sh
```

### From Your Client Machine

**Step 5: Connect via RDP**
- Connect to `192.168.10.205:3389`
- **DO NOT MOVE MOUSE** once connected
- Let it run for ~20-30 seconds

**Step 6: Disconnect and collect log**

### Cleanup on Server
```bash
ssh greg@192.168.10.205
kill $(cat /tmp/unclutter.pid)
```

---

## Method 2: Position Mouse Then Don't Move It

### On the Test Server (192.168.10.205)

**Step 1: SSH to server**
```bash
ssh greg@192.168.10.205
```

**Step 2: Start the RDP server**
```bash
cd ~
./run-server.sh
```

### From Your Client Machine

**Step 3: Connect via RDP**
- Connect to `192.168.10.205:3389`

**Step 4: Position mouse to a stable location**
- Move mouse to center of screen (640, 400) - the purple wallpaper area
- This position is stable (we verified it)
- **STOP MOVING COMPLETELY**
- Keep hands off mouse/touchpad
- Let it run for ~20-30 seconds

**Step 5: Disconnect**

---

## Method 3: Automated Test (Most Reliable)

This uses xdotool to position the mouse programmatically, then captures frames.

### On the Test Server (192.168.10.205)

**Create test script:**
```bash
cat > ~/test-static.sh << 'EOF'
#!/bin/bash

# Install xdotool if needed
which xdotool >/dev/null || sudo apt-get install -y xdotool

# Position mouse at center of screen (stable purple wallpaper)
export DISPLAY=:0
xdotool mousemove 640 400

echo "Mouse positioned at center (640, 400)"
echo "Starting RDP server..."
echo "Connect via RDP and DO NOT TOUCH MOUSE"
echo ""

# Start server
cd ~
./run-server.sh
EOF

chmod +x ~/test-static.sh
```

**Run the test:**
```bash
ssh greg@192.168.10.205
./test-static.sh
```

**From client**: Connect, don't touch anything, wait 30 seconds, disconnect.

---

## What to Look For in Logs

After the test, analyze the log:

```bash
# Copy log
scp greg@192.168.10.205:~/colorful-test-TIMESTAMP.log ./

# Check if cycling position is now stable
rg "🔍 CYCLING POSITION" colorful-test-TIMESTAMP.log

# Expected with static screen:
# All frames should show SAME BGRA value

# Check if we get temporal stability
rg "TEMPORAL STABLE" colorful-test-TIMESTAMP.log

# Expected with static screen:
# Should see "✅ TEMPORAL STABLE" on most/all frames
```

---

## Success Criteria

### If Screen is Truly Static:

**Expected**: All frames should have identical auxiliary buffers
```
✅ TEMPORAL STABLE: Auxiliary IDENTICAL
✅ TEMPORAL STABLE: Auxiliary IDENTICAL
✅ TEMPORAL STABLE: Auxiliary IDENTICAL
```

**Cycling position should not cycle:**
```
🔍 CYCLING POSITION (329,122): BGRA=(36, 10, 48) → YUV=(20, 137, 146)
🔍 CYCLING POSITION (329,122): BGRA=(36, 10, 48) → YUV=(20, 137, 146)
🔍 CYCLING POSITION (329,122): BGRA=(36, 10, 48) → YUV=(20, 137, 146)
```

### If We Still See Cycling:

**Then we have nondeterminism** in:
- PipeWire frame capture
- Color conversion (SIMD)
- Or something else we haven't found

---

## Next Steps Based on Results

### Result A: Screen is Stable, Aux Buffers Stable
- **Conclusion**: Input was changing (cursor/UI)
- **Next**: Re-enable P-frames, test with static screen
- **If P-frames work**: Problem solved! Just need stable input
- **If P-frames fail**: Dual-stream coordination issue

### Result B: Screen is Stable, Aux Buffers Still Cycle
- **Conclusion**: Real nondeterminism in our pipeline
- **Next**: Deep dive into PipeWire or color conversion
- **Focus**: SIMD/AVX2 code or PipeWire frame delivery

---

## My Recommendation: Use Method 3 (Automated)

Most reliable way to ensure mouse doesn't move:
1. Programmatically position it
2. Start server
3. Connect and don't touch anything
4. Analyze results

This eliminates human error (accidental mouse movement).
