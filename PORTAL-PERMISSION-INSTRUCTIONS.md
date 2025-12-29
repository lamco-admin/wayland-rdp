# Portal Permission Dialog Instructions

**Based on your screenshots** (Screenshot 287 and 288)

---

## WHAT TO SELECT IN THE DIALOG

### Dialog 1: "Share screen anonymously" (Screenshot 287)

**Simply click**: ✅ **"Share"** button (bottom right, orange)

Nothing else to configure on this screen - just click Share.

---

### Dialog 2: Screen Selection (Screenshot 288)

**This is the critical one!**

**What you should see**:
- A list or grid showing available screens to share
- In your case with Weston: Should show the weston window/outputs

**What to select**:

**Option A: If you see "Entire Screen" or similar**:
- ✅ SELECT THIS if it's the weston window
- This should capture everything weston is showing

**Option B: If you see multiple windows listed**:
- ✅ Look for "Weston" or the weston window
- ✅ SELECT the weston compositor window
- NOT individual apps inside weston

**Option C: If you see "Select All" or checkboxes**:
- ✅ Check all available outputs
- ✅ Make sure to include the weston display

**Also check** (if visible):
- ✅ Keyboard checkbox (for input control)
- ✅ Pointer checkbox (for mouse control)

**Then click**: ✅ **"Share"** button

---

## THE ISSUE YOU'RE HITTING

**Problem**: When you select "entire screen" you're probably selecting:
- Your GNOME desktop (wayland-0)
- NOT the weston compositor (wayland-1)

**Result**: Portal provides 1 stream (GNOME desktop), not weston's outputs

---

## CORRECT APPROACH

**Look in the dialog for**:
- Something labeled "Weston"
- Or the weston window specifically
- It might be in a different tab/section

**In Screenshot 288**, I can see there's a selection area. **Can you tell me**:
1. What options are shown in that dialog?
2. Is there a window/application list?
3. Do you see "Weston" anywhere?
4. What does it say at the top of the selection area?

**This will help me guide you to the correct selection!**

---

## ALTERNATIVE: Try Window Mode

If "entire screen" only shows GNOME desktop:

**In the Portal dialog**, look for:
- "Window" mode instead of "Screen" mode
- Tab that says "Windows" or "Applications"
- List of running applications

**Then**:
- ✅ Find "Weston" in the window list
- ✅ Select the Weston compositor window
- ✅ Click Share

**This should capture weston's outputs specifically!**

---

## DEBUGGING

**After you click Share**, the log will show:

**If working**:
```
📺 Portal provided stream: node_id=X, size=(3840, 1080), position=(0, 0)
```

Or if weston creates 2 separate outputs:
```
📺 Portal provided stream: node_id=X, size=(1920, 1080), position=(0, 0)
📺 Portal provided stream: node_id=Y, size=(1920, 1080), position=(1920, 0)
📊 Total streams from Portal: 2
```

**If not working** (what you're seeing now):
```
📺 Portal provided stream: node_id=X, size=(1280, 800), position=(0, 0)
📊 Total streams from Portal: 1
```

This means Portal captured GNOME desktop, not weston.

---

## SEND ME

After you grant permission:

1. The server log (weston-multimon-test-*.log)
2. Description of what was in that selection dialog
3. What you actually selected

Then I can diagnose exactly what's happening!

**Or better yet**: Can you take a screenshot showing what options are in the selection area of Screenshot 288? That would help me tell you exactly what to click!
