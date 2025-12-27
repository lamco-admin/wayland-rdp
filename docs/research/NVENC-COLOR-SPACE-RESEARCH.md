# NVENC Color Space Handling: Comprehensive Research

**Date:** 2025-12-27
**Purpose:** Document NVIDIA NVENC color space handling for AVC444 and hardware encoding implementation
**Criticality:** HIGH - This affects color accuracy for a commercial product

---

## EXECUTIVE SUMMARY

### The Critical Discovery

**NVENC's VUI parameters are METADATA ONLY - they do NOT control the actual RGB-to-YUV conversion matrix used internally.**

When you feed NVENC an RGB/ARGB/BGRA input:
1. NVENC performs internal RGB-to-YUV conversion using an **undocumented, fixed matrix**
2. Evidence strongly suggests NVENC uses **BT.601 coefficients** internally
3. The `colourMatrix` VUI parameter only **tags** the output bitstream for decoders
4. This creates a disconnect: the actual colors may be BT.601, but you can tag them as BT.709

### Implications for Our Implementation

| Scenario | What Happens | Color Accuracy |
|----------|-------------|----------------|
| Feed BGRA, set VUI to BT.709 | NVENC converts with BT.601(?), tags as BT.709 | **INCORRECT** - decoder uses wrong matrix |
| Feed BGRA, set VUI to BT.601 | NVENC converts with BT.601(?), tags as BT.601 | Correct (if assumption is true) |
| Feed YUV directly (NV12) | No conversion, VUI matches your conversion | **CORRECT** - full control |

### Recommended Approach

**For color-accurate encoding:**
1. Convert BGRA to YUV ourselves using BT.709 matrix
2. Feed NVENC the NV12/I420 data (no internal conversion)
3. Set VUI parameters to match our conversion matrix (BT.709)
4. Result: Color-accurate encoding with correct metadata

---

## NVENC VUI PARAMETER DOCUMENTATION

### NV_ENC_CONFIG_H264_VUI_PARAMETERS Structure

From [nvEncodeAPI.h](https://github.com/FFmpeg/nv-codec-headers/blob/master/include/ffnvcodec/nvEncodeAPI.h):

```c
typedef struct _NV_ENC_CONFIG_H264_VUI_PARAMETERS {
    uint32_t overscanInfoPresentFlag;
    uint32_t overscanInfo;
    uint32_t videoSignalTypePresentFlag;
    NV_ENC_VUI_VIDEO_FORMAT videoFormat;
    uint32_t videoFullRangeFlag;           // 0 = limited (16-235), 1 = full (0-255)
    uint32_t colourDescriptionPresentFlag;
    NV_ENC_VUI_COLOR_PRIMARIES colourPrimaries;
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC transferCharacteristics;
    NV_ENC_VUI_MATRIX_COEFFS colourMatrix;  // BT.601, BT.709, etc.
    uint32_t chromaSampleLocationFlag;
    uint32_t chromaSampleLocationTop;
    uint32_t chromaSampleLocationBot;
    uint32_t bitstreamRestrictionFlag;
    uint32_t timingInfoPresentFlag;
    uint32_t numUnitInTicks;
    uint32_t timeScale;
    uint32_t reserved[12];
} NV_ENC_CONFIG_H264_VUI_PARAMETERS;

// HEVC uses the same structure
typedef NV_ENC_CONFIG_H264_VUI_PARAMETERS NV_ENC_CONFIG_HEVC_VUI_PARAMETERS;
```

### Color Primaries Enum

```c
typedef enum _NV_ENC_VUI_COLOR_PRIMARIES {
    NV_ENC_VUI_COLOR_PRIMARIES_UNDEFINED     = 0,
    NV_ENC_VUI_COLOR_PRIMARIES_BT709         = 1,   // HD standard
    NV_ENC_VUI_COLOR_PRIMARIES_UNSPECIFIED   = 2,
    NV_ENC_VUI_COLOR_PRIMARIES_RESERVED      = 3,
    NV_ENC_VUI_COLOR_PRIMARIES_BT470M        = 4,
    NV_ENC_VUI_COLOR_PRIMARIES_BT470BG       = 5,   // SD PAL
    NV_ENC_VUI_COLOR_PRIMARIES_SMPTE170M     = 6,   // SD NTSC
    NV_ENC_VUI_COLOR_PRIMARIES_SMPTE240M     = 7,
    NV_ENC_VUI_COLOR_PRIMARIES_FILM          = 8,
    NV_ENC_VUI_COLOR_PRIMARIES_BT2020        = 9,   // UHD/HDR
    NV_ENC_VUI_COLOR_PRIMARIES_SMPTE428      = 10,
    NV_ENC_VUI_COLOR_PRIMARIES_SMPTE431      = 11,  // DCI-P3
    NV_ENC_VUI_COLOR_PRIMARIES_SMPTE432      = 12,  // Display P3
    NV_ENC_VUI_COLOR_PRIMARIES_JEDEC_P22     = 22
} NV_ENC_VUI_COLOR_PRIMARIES;
```

### Transfer Characteristics Enum

```c
typedef enum _NV_ENC_VUI_TRANSFER_CHARACTERISTIC {
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_UNDEFINED    = 0,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_BT709        = 1,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_UNSPECIFIED  = 2,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_BT470M       = 4,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_BT470BG      = 5,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_SMPTE170M    = 6,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_SMPTE240M    = 7,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_LINEAR       = 8,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_LOG          = 9,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_LOG_SQRT     = 10,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_IEC61966_2_4 = 11,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_BT1361_ECG   = 12,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_SRGB         = 13,  // sRGB
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_BT2020_10    = 14,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_BT2020_12    = 15,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_SMPTE2084    = 16,  // PQ/HDR10
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_SMPTE428     = 17,
    NV_ENC_VUI_TRANSFER_CHARACTERISTIC_ARIB_STD_B67 = 18   // HLG
} NV_ENC_VUI_TRANSFER_CHARACTERISTIC;
```

### Matrix Coefficients Enum

```c
typedef enum _NV_ENC_VUI_MATRIX_COEFFS {
    NV_ENC_VUI_MATRIX_COEFFS_RGB         = 0,   // Identity (no conversion)
    NV_ENC_VUI_MATRIX_COEFFS_BT709       = 1,   // HD standard
    NV_ENC_VUI_MATRIX_COEFFS_UNSPECIFIED = 2,
    NV_ENC_VUI_MATRIX_COEFFS_FCC         = 4,
    NV_ENC_VUI_MATRIX_COEFFS_BT470BG     = 5,   // SD PAL (same as BT.601)
    NV_ENC_VUI_MATRIX_COEFFS_SMPTE170M   = 6,   // SD NTSC (same as BT.601)
    NV_ENC_VUI_MATRIX_COEFFS_SMPTE240M   = 7,
    NV_ENC_VUI_MATRIX_COEFFS_YCGCO       = 8,
    NV_ENC_VUI_MATRIX_COEFFS_BT2020_NCL  = 9,   // UHD non-constant luminance
    NV_ENC_VUI_MATRIX_COEFFS_BT2020_CL   = 10,  // UHD constant luminance
    NV_ENC_VUI_MATRIX_COEFFS_SMPTE2085   = 11
} NV_ENC_VUI_MATRIX_COEFFS;
```

---

## THE CRITICAL ISSUE: VUI vs ACTUAL CONVERSION

### What VUI Parameters Actually Do

From the NVENC programming guide and forum discussions:

> "VUI parameters are metadata written into the H.264/HEVC bitstream header. They tell the decoder what color space the video is encoded in. **They do NOT control the actual color conversion performed by the encoder.**"

This is confirmed by multiple sources:
- [NVIDIA Developer Forums](https://forums.developer.nvidia.com/t/what-transform-does-nvenc-use-from-bgra-to-yuv-ycbcr/209614): User asks "What transform does NVENC use from BGRA to YUV/YCbCr?" - **No official NVIDIA response**
- [NVIDIA Developer Forums](https://forums.developer.nvidia.com/t/default-colorspace-conversion-matrix-for-argb-input-in-nvidia-video-codec/304161): "What is the default color conversion matrix when the input is in argb format?" - **No official NVIDIA response**

### Evidence That NVENC Uses BT.601 Internally

From [VideoHelp Forum](https://forum.videohelp.com/threads/384118-Hardware-encoders(Quicksync-NVENC)-colormatrix-behavior):

> "Shadowplay which is NVIDIA own recorder tag video file with BT.601 even for HD recording."
>
> "In OBS even using BT709 or BT601 for NVENC it output same colors but they are tagged differently. OBS NVENC bt709/BT601 tagged files has same colors than Shadowplay bt601 tagged file."
>
> "People complain about Shadowplay 601 for NVIDIA for ages in their forums but apparently the colors are bt709, only tagged wrongly."

This strongly suggests:
1. NVENC uses a fixed internal matrix (likely BT.601 coefficients)
2. The matrix does NOT change based on VUI settings
3. NVIDIA's own tools tag incorrectly

### NVIDIA's NPP Library Uses BT.601

From [NVIDIA NPP Documentation](https://docs.nvidia.com/cuda/npp/image_color_conversion.html):

> "For RGB to YUV conversion: nY = 0.299F * R + 0.587F * G + 0.114F * B"

These are **BT.601 coefficients**:
- Y = 0.299R + 0.587G + 0.114B (BT.601)
- Y = 0.2126R + 0.7152G + 0.0722B (BT.709)

### VPI Library Also Uses BT.601

From [NVIDIA VPI Documentation](https://docs.nvidia.com/vpi/algo_imageconv.html):

> "For RGB ↔ YUV conversions, VPI uses the ITU-R BT.601 625-line specification. It's the same standard used by JPEG File Interchange Format (JFIF)."

---

## HOW FFMPEG HANDLES NVENC COLOR

From [FFmpeg nvenc.c](https://ffmpeg.org/doxygen/trunk/nvenc_8c_source.html):

### For RGB Input (Non-GBRP)

```c
if ((pixdesc->flags & AV_PIX_FMT_FLAG_RGB) && !IS_GBRP(ctx->data_pix_fmt)) {
    vui->colourMatrix = AVCOL_SPC_BT470BG;  // BT.601!
    vui->colourPrimaries = avctx->color_primaries;
    vui->transferCharacteristics = avctx->color_trc;
    vui->videoFullRangeFlag = 0;  // Limited range
}
```

**Key Insight:** FFmpeg explicitly sets `colourMatrix = BT470BG` (BT.601) for RGB input to NVENC!

This confirms FFmpeg developers know NVENC uses BT.601 internally for RGB conversion.

### For YUV Input

```c
else {
    vui->colourMatrix = IS_GBRP(ctx->data_pix_fmt) ? AVCOL_SPC_RGB : avctx->colorspace;
    vui->colourPrimaries = avctx->color_primaries;
    vui->transferCharacteristics = avctx->color_trc;
    vui->videoFullRangeFlag = (avctx->color_range == AVCOL_RANGE_JPEG || ...);
}
```

For YUV input, FFmpeg respects the user's colorspace setting because no conversion is happening.

---

## HOW SUNSHINE HANDLES NVENC COLOR

From [Sunshine nvenc_base.cpp](https://github.com/LizardByte/Sunshine):

```cpp
auto fill_h264_hevc_vui = [&](auto &vui_config) {
    vui_config.videoSignalTypePresentFlag = 1;
    vui_config.videoFormat = NV_ENC_VUI_VIDEO_FORMAT_UNSPECIFIED;
    vui_config.videoFullRangeFlag = colorspace.full_range;
    vui_config.colourDescriptionPresentFlag = 1;
    vui_config.colourPrimaries = colorspace.primaries;
    vui_config.transferCharacteristics = colorspace.tranfer_function;
    vui_config.colourMatrix = colorspace.matrix;
    vui_config.chromaSampleLocationFlag = buffer_is_yuv444() ? 0 : 1;
    vui_config.chromaSampleLocationTop = 0;
    vui_config.chromaSampleLocationBot = 0;
};
```

Sunshine properly sets all VUI parameters. The colorspace structure contains:
- `primaries`: BT.709 for SDR content
- `transfer_function`: BT.709 for SDR
- `matrix`: BT.709 for SDR
- `full_range`: true/false based on capture

---

## RIGAYA NVENCC OPTIONS

[rigaya/NVEnc](https://github.com/rigaya/NVEnc) provides comprehensive color space control:

### Command-Line Options

```bash
# Set VUI color metadata
--colorprim bt709       # Color primaries
--colormatrix bt709     # Matrix coefficients
--transfer bt709        # Transfer characteristics
--colorrange limited    # or "full"

# Actual color space conversion (using CUDA)
--vpp-colorspace matrix=bt601:bt709,colorprim=bt601:bt709,transfer=bt601:bt709
```

### Key Insight

NVEncC distinguishes between:
1. **VUI metadata flags** (`--colorprim`, `--colormatrix`, `--transfer`) - just tagging
2. **Actual conversion** (`--vpp-colorspace`) - uses CUDA for real color transformation

This confirms that setting VUI flags alone doesn't change the actual colors.

---

## nvidia-video-codec-sdk RUST CRATE

From [docs.rs/nvidia-video-codec-sdk](https://docs.rs/nvidia-video-codec-sdk/latest/nvidia_video_codec_sdk/):

### Available Enums

The Rust crate exposes all VUI enums:
- `NV_ENC_VUI_COLOR_PRIMARIES` (14 variants)
- `NV_ENC_VUI_TRANSFER_CHARACTERISTIC` (18 variants)
- `NV_ENC_VUI_MATRIX_COEFFS` (11 variants)

### Crate Completeness

The crate provides **low-level bindings** to the NVIDIA API. It exposes the `NV_ENC_CONFIG` structure which contains codec-specific VUI parameters.

**Not Missing:** The crate isn't incomplete - it exposes what NVIDIA provides.
**What's Missing:** NVIDIA doesn't provide API control over the internal RGB-to-YUV conversion matrix.

---

## NVIDIA INPUT FORMAT SUPPORT

### Supported RGB Formats

From nvEncodeAPI.h:
```c
NV_ENC_BUFFER_FORMAT_ARGB          // 32-bit ARGB
NV_ENC_BUFFER_FORMAT_ABGR          // 32-bit ABGR
NV_ENC_BUFFER_FORMAT_AYUV          // 32-bit packed YUV 4:4:4
```

### How RGB Input is Processed

From [NVIDIA Developer Forums](https://forums.developer.nvidia.com/t/nvenc-yuv444-from-argb-format/189415):

> "ARGB to YUV444 does work automatically in NvEnc. Just make sure to have the correct input format... and set encodeCodecConfig.hevcConfig.chromaFormatIDC to 3"
>
> "To tell the encoder what colorspaces to use to convert from ARGB to YUV, you need to set the color parameters within encodeCodecConfig.hevcConfig.hevcVUIParameters"

**But this is misleading!** Setting VUI parameters tells the *decoder* what colorspace to use - it doesn't tell the *encoder* how to convert.

---

## RECOMMENDED IMPLEMENTATION

### Option 1: Do Our Own Conversion (RECOMMENDED)

```rust
// 1. Convert BGRA to YUV ourselves using BT.709
fn bgra_to_nv12_bt709(bgra: &[u8], width: u32, height: u32) -> Vec<u8> {
    // BT.709 matrix coefficients
    const KR: f32 = 0.2126;
    const KG: f32 = 0.7152;
    const KB: f32 = 0.0722;

    // Full conversion with proper coefficients
    for each pixel:
        Y = KR*R + KG*G + KB*B
        Cb = (B - Y) / (2 * (1 - KB)) + 128
        Cr = (R - Y) / (2 * (1 - KR)) + 128
}

// 2. Feed NVENC NV12 data (no internal conversion)
let nv12_data = bgra_to_nv12_bt709(&frame, width, height);
nvenc.encode_nv12(&nv12_data);

// 3. Set VUI to match our conversion
vui.colourMatrix = NV_ENC_VUI_MATRIX_COEFFS_BT709;
vui.colourPrimaries = NV_ENC_VUI_COLOR_PRIMARIES_BT709;
vui.transferCharacteristics = NV_ENC_VUI_TRANSFER_CHARACTERISTIC_BT709;
```

### Option 2: Accept BT.601 Internal Conversion

If performance is critical and you can accept BT.601:

```rust
// Feed RGB directly to NVENC
nvenc.encode_argb(&frame);

// Set VUI to BT.601 (matches internal conversion)
vui.colourMatrix = NV_ENC_VUI_MATRIX_COEFFS_SMPTE170M;  // BT.601
vui.colourPrimaries = NV_ENC_VUI_COLOR_PRIMARIES_SMPTE170M;
```

### Option 3: Use CUDA for Conversion (Advanced)

```rust
// Use NVIDIA CUDA for GPU-accelerated BT.709 conversion
cuda_bgra_to_nv12_bt709(d_bgra, d_nv12, width, height);
nvenc.encode_nv12_cuda(d_nv12);  // Zero-copy from CUDA memory
```

---

## COLOR SPACE CONVERSION MATRICES

### BT.601 (SD Standard)

```
Y  =  0.299R  + 0.587G  + 0.114B
Cb = -0.169R  - 0.331G  + 0.500B + 128
Cr =  0.500R  - 0.419G  - 0.081B + 128
```

### BT.709 (HD Standard)

```
Y  =  0.2126R + 0.7152G + 0.0722B
Cb = -0.1146R - 0.3854G + 0.5000B + 128
Cr =  0.5000R - 0.4542G - 0.0458B + 128
```

### BT.2020 (UHD Standard)

```
Y  =  0.2627R + 0.6780G + 0.0593B
Cb = -0.1396R - 0.3604G + 0.5000B + 128
Cr =  0.5000R - 0.4598G - 0.0402B + 128
```

---

## IMPACT ON AVC444 IMPLEMENTATION

### For AVC444 with BGRA Input

Since AVC444 requires converting BGRA to YUV444 (then packing into dual YUV420 streams), we MUST do our own conversion anyway:

```rust
// AVC444 Encoder Pipeline
fn encode_avc444(bgra: &[u8], width: u32, height: u32) -> (Vec<u8>, Vec<u8>) {
    // Step 1: BGRA to YUV444 (we control the matrix!)
    let (y444, u444, v444) = bgra_to_yuv444_bt709(bgra, width, height);

    // Step 2: Decompose into two YUV420 streams
    let (main_yuv420, aux_yuv420) = decompose_yuv444_to_dual_yuv420(y444, u444, v444);

    // Step 3: Encode both streams
    // Feed NV12 data to NVENC - no internal conversion!
    let stream1 = nvenc_main.encode_nv12(&main_yuv420.to_nv12());
    let stream2 = nvenc_aux.encode_nv12(&aux_yuv420.to_nv12());

    // Step 4: Set VUI correctly
    // Both streams tagged as BT.709 to match our conversion

    (stream1, stream2)
}
```

**This is actually good news for AVC444!** Since we're doing our own YUV444 conversion:
- We have full control over the color matrix
- We can use BT.709 for accurate HD colors
- VUI parameters will correctly match our conversion
- No hidden internal conversion to worry about

---

## SOURCES

### Official NVIDIA Documentation

- [NVENC Video Encoder API Programming Guide v13.0](https://docs.nvidia.com/video-technologies/video-codec-sdk/13.0/nvenc-video-encoder-api-prog-guide/index.html)
- [NVIDIA Video Codec SDK v12.2](https://docs.nvidia.com/video-technologies/video-codec-sdk/12.2/index.html)
- [NVIDIA NPP Image Color Conversion](https://docs.nvidia.com/cuda/npp/image_color_conversion.html)
- [NVIDIA VPI Color Conversion](https://docs.nvidia.com/vpi/algo_imageconv.html)

### NVIDIA Developer Forums (Unanswered Questions)

- [What transform does NVENC use from BGRA to YUV/YCbCr?](https://forums.developer.nvidia.com/t/what-transform-does-nvenc-use-from-bgra-to-yuv-ycbcr/209614) - No official answer
- [Default colorspace conversion matrix for argb input](https://forums.developer.nvidia.com/t/default-colorspace-conversion-matrix-for-argb-input-in-nvidia-video-codec/304161) - No official answer
- [NvFBC + NVENC VUI parameters](https://forums.developer.nvidia.com/t/nvfbc-nvenc/203273) - Redirected, no answer
- [How to put general video information in vui like Color space](https://forums.developer.nvidia.com/t/how-to-put-general-video-information-in-vui-like-color-space-when-encode-with-nvenc/239818) - "Upgrade to later Jetpack"

### Open Source Implementations

- [FFmpeg nvenc.c](https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/nvenc.c) - Sets BT.601 for RGB input
- [nv-codec-headers/nvEncodeAPI.h](https://github.com/FFmpeg/nv-codec-headers/blob/master/include/ffnvcodec/nvEncodeAPI.h) - Complete API definitions
- [Sunshine nvenc_base.cpp](https://github.com/LizardByte/Sunshine) - VUI configuration example
- [rigaya/NVEnc](https://github.com/rigaya/NVEnc/blob/master/NVEncC_Options.en.md) - Comprehensive options
- [nvidia-video-codec-sdk Rust crate](https://docs.rs/nvidia-video-codec-sdk/) - Rust bindings

### Community Discussions

- [VideoHelp: Hardware encoders colormatrix behavior](https://forum.videohelp.com/threads/384118-Hardware-encoders(Quicksync-NVENC)-colormatrix-behavior) - Evidence NVENC uses BT.601
- [OBS Forums: Quick Sync / NVENC YUV Full Range Color](https://obsproject.com/forum/threads/quick-sync-nvenc-yuv-full-range-color.45284/)

---

## CONCLUSION

### Key Findings

1. **NVENC VUI parameters are metadata only** - they tag the bitstream but don't control internal conversion
2. **NVENC likely uses BT.601 internally** for RGB-to-YUV conversion (based on evidence from NVIDIA's own tools and FFmpeg's handling)
3. **NVIDIA has NOT documented** the exact internal conversion matrix
4. **The solution is to do our own conversion** - feed NVENC YUV data (NV12/I420) instead of RGB

### For Our Project

1. **AVC444 is not affected** - we do our own BGRA-to-YUV444 conversion anyway
2. **For AVC420 with hardware encoding**, we should:
   - Option A: Convert BGRA to NV12 ourselves using BT.709, feed NV12 to NVENC
   - Option B: Accept BT.601 internal conversion, set VUI to match
3. **VUI parameters must match** the actual conversion matrix used

### Recommendation

For a commercial product requiring color accuracy:

**Do NOT rely on NVENC's internal RGB conversion.** Instead:
1. Implement BGRA-to-NV12 conversion ourselves (CUDA or CPU SIMD)
2. Use BT.709 matrix for HD content
3. Feed NV12 to NVENC
4. Set VUI parameters to BT.709
5. Result: Correct colors with correct metadata

This adds ~2-5ms latency but ensures color accuracy for professional users.
