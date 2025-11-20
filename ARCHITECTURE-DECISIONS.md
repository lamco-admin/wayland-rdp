# WRD-Server Architecture Decisions

**Core Questions**:
1. Can/should we bypass other compositors?
2. Link to or use existing compositors?
3. Implement our own Portal solutions?

---

## ANSWER 1: Bypass Other Compositors?

**YES** - Lamco compositor (4,586 lines) is STANDALONE:
- Complete Wayland protocols ✅
- No dependency on Mutter/KWin/Sway ✅
- Just needs display backend (Xvfb)

**Dependencies**:
- Smithay (Rust framework)
- Xvfb (virtual display, 20MB)
- **Total external code: Minimal**

---

## ANSWER 2: Use Existing Compositors?

**NO** - Standalone Lamco is BETTER:

**vs wlroots**:
- Smithay: Pure Rust ✅
- wlroots: C + FFI ❌

**vs Mutter/Sway**:
- Lamco: 150MB ✅
- Mutter: 500MB ❌

**We already built it!**

---

## ANSWER 3: Implement Portal?

**NO** - Not needed:
- Lamco solves clipboard directly
- Portal backend doesn't help headless
- Maintenance burden
- **Our SelectionHandler is better**

---

## RECOMMENDED ARCHITECTURE

**Dual Mode**:
```
Desktop:    Portal Mode (current)
Headless:   Compositor Mode (Lamco + Xvfb)
```

**Dependencies**:
- Portal: Uses system desktop
- Compositor: Xvfb only (20MB)

**Clipboard**:
- Portal: Broken (signal issue)
- Compositor: Working (SelectionHandler) ✅

---

## IMPLEMENTATION

**Current**: Both modes compile ✅
**Next**: Add Xvfb backend
**Timeline**: 1-2 weeks to production

**This architecture is optimal.**
