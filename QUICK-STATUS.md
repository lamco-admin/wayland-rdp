# Quick Status - AVC444 Investigation
**Date**: 2025-12-27 23:04

## ✅ What Works
- **AVC420**: Perfect (colors, performance, no corruption)
- **AVC444 both all-I**: Perfect (proves packing is correct, but too slow)
- **AVC444 main all-I + aux P**: Readable, no lavender (but colors wrong)

## ❌ What's Broken
- **AVC444 with any main P-frames**: Lavender corruption
- **AVC444 colors**: Always slightly off (even without corruption)

## 🔧 Currently Deployed
**MD5**: `b37d8a07ea27da8274fd4a7597297bff`
**Config**: AVC444, Main all-I, Aux normal P, BT.709
**Result**: Readable, no corruption, colors wrong

## 🎯 What We Know FOR SURE
1. Auxiliary packing algorithm is 100% correct (matches FreeRDP, proven by all-I test)
2. Main view is correct (proven by all-I test)
3. Color conversion BGRA→YUV444 is correct (AVC420 works perfectly)
4. Main P-frames + auxiliary = corruption
5. Main all-I + auxiliary = no corruption but wrong colors
6. SIMD is not the issue (tested without, still wrong)
7. U/V are not swapped (tested, made it worse)
8. Padding helps but doesn't fix it

## 🚀 Next Steps
1. **Log colorful areas not just gray** - current logs only show neutral U=V=128
2. **Compare AVC420 vs AVC444 decoded output** side-by-side with same input
3. **Check if auxiliary reconstruction formula is wrong** on client side
4. **Test auxiliary with different sample positions** (maybe we're off by 1 pixel?)
5. **Research if Windows client expects specific encoder settings** for main stream

## 📝 Key Files Modified
- `src/egfx/avc444_encoder.rs:325` - Force main all-I
- `src/egfx/yuv444_packing.rs:419-445` - Deterministic chroma padding
- `src/egfx/color_convert.rs:209,676` - SIMD tests
- Logs: `/tmp/rdp-analysis.log` (local copy), `~/kde-test-*.log` (server)

## 🔍 Critical Insight
The desktop background (0,0) is gray (BGRA=19,19,19 → U=V=128 neutral).
Need to log from **colorful areas** to see what's actually wrong with chroma!
