# Enhanced Diagnostic Build - 2025-12-27

## What Was Done

### Enhanced Multi-Position Logging

Added comprehensive diagnostic logging that samples **multiple screen positions** instead of just (0,0):

#### 1. Color Conversion Logging (`color_convert.rs`)
Samples 5 positions across screen during BGRA→YUV444 conversion:
- Top-left (0,0)
- Center (640,400)
- Top-right area (1000,100)
- Bottom-left (200,600)
- Middle-right (800,300)

Shows: `BGRA=(B,G,R) → YUV=(Y,U,V)` for each position

#### 2. Main View Packing Logging (`yuv444_packing.rs`)
Samples 5 strategic positions:
- Top-left, top-right, center, bottom-left, bottom-right
- Shows YUV444 input (2x2 blocks)
- Shows subsampled U420/V420 output
- **Purpose**: Verify 4:2:0 subsampling is correct

#### 3. Auxiliary View Packing Logging (`yuv444_packing.rs`)
Samples 3 positions with detailed analysis:
- Shows auxiliary Y plane data (should contain U444/V444 odd rows)
- Shows auxiliary U/V chroma (B6/B7 blocks)
- Shows source data from YUV444 for comparison
- **Purpose**: Verify auxiliary packing matches MS-RDPEGFX spec

## Deployed Binary

**Location**: `greg@192.168.10.205:~/lamco-rdp-server`
**MD5**: `4145bfac348192ec44b0ba57c9cbadef`
**Config**: AVC444, main all-I, aux P-frames, BT.709

## Testing Instructions

### 1. Prepare Colorful Screen Content

**CRITICAL**: The desktop background is likely gray (BGRA ~19,19,19 → U=V=128 neutral).

Before running the server, **open some colorful windows**:
- Web browser with colorful webpage (reds, blues, greens)
- Image viewer with colorful photo
- Color picker app or settings with bright colors
- Terminal with colored text

This ensures we sample **actual color data** not just neutral gray!

### 2. Run Server with DEBUG Logging

```bash
ssh greg@192.168.10.205
cd ~
./lamco-rdp-server -c config.toml -vvv 2>&1 | tee ~/colorful-test-$(date +%Y%m%d-%H%M%S).log
```

### 3. Connect and Capture Logs

Connect from Windows RDP client and look for these log sections:

```
🎨 COLOR CONVERSION SAMPLES (Matrix::BT709):
  center (640,400): BGRA=(XXX,XXX,XXX) → YUV=(YYY,UUU,VVV)
  ...

═══ MAIN VIEW MULTI-POSITION ANALYSIS ═══
📍 Center @ (640, 400)
  Y444: [YYY,YYY] [YYY,YYY]
  U444: [UUU,UUU] [UUU,UUU]
  V444: [VVV,VVV] [VVV,VVV]
  Main U420: UUU, Main V420: VVV

═══ AUXILIARY VIEW MULTI-POSITION ANALYSIS ═══
📍 Center @ (640, 400)
  Aux Y (row 400): [AAA,AAA,AAA,AAA]
  Source U444[row 401]: [UUU,UUU,UUU,UUU]
  ...
```

### 4. What to Look For

**Key Questions**:
1. **Are U/V values actually varying?** (not all 128)
   - Gray: U=V≈128
   - Red: V>128, U<128
   - Blue: U>128, V<128
   - Green: U<128, V<128

2. **Is main view subsampling preserving chroma?**
   - Compare Y444 U/V vs Main U420/V420
   - Should be averaged (box filter)

3. **Is auxiliary packing capturing odd samples?**
   - Aux Y should contain U444/V444 odd rows
   - Aux U/V should contain odd-column samples

4. **Temporal stability**
   - Does `TEMPORAL STABLE` appear? (frame-to-frame consistency)
   - Or `TEMPORAL CHANGE`? (unexpected variation)

## Expected Output

### If Colors Are Neutral Gray
```
center (640,400): BGRA=(19,19,19) → YUV=(19,128,128)
```
→ **Open colorful windows and retest!**

### If Colors Are Varied
```
center (640,400): BGRA=(255,50,50) → YUV=(81,90,178)  # Reddish
top-right (1000,100): BGRA=(50,50,255) → YUV=(81,198,90)  # Bluish
```
→ **Perfect! Now we can analyze what's wrong with chroma**

## Next Steps After Log Collection

1. **Find non-neutral chroma**: Look for U/V values != 128
2. **Compare main vs auxiliary**: See if auxiliary is capturing correct residuals
3. **Check for U/V swap**: If red looks blue-ish, U and V might be swapped
4. **Analyze temporal changes**: P-frame corruption shows as hash changes

## Files Modified

- `src/egfx/color_convert.rs`: Multi-position BGRA→YUV444 sampling
- `src/egfx/yuv444_packing.rs`: Multi-position main/aux analysis

---

**Status**: Ready for testing with colorful screen content! 🎨
