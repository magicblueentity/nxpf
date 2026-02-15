# NXPF
**N**ew E**x**tremely **P**ortable Image **F**ormat.

A lightweight, binary image format designed for efficiency and portability.

Original design created by **facedev**. See [bruh](https://github.com/face-hh/bruh) for more.

## Format Specification

**Header (14 bytes):**
- Magic: `NXPF` (4 bytes)
- Width: u32 little-endian
- Height: u32 little-endian  
- Flags: 1 byte (bit 0: has alpha, bit 1: compressed)
- Compression type: 1 byte (0=none, 1=deflate)

**Pixel Data:**
- RGB: 3 bytes per pixel (uncompressed) or compressed
- RGBA: 4 bytes per pixel (uncompressed) or compressed

## How to Use

1. Clone the repo and navigate to the directory
2. To convert PNG to NXPF: `cargo run compile path/to/image.png`
3. To view NXPF file: `cargo run path/to/image.nxpf`

## File Size Comparison
- PNG: ~5-50 KB (compressed)
- Old format (BRUH): ~2-4 MB (uncompressed hex text)
- NXPF uncompressed: ~900 KB (3 bytes/pixel)
- NXPF compressed: ~100-500 KB (with deflate)

## Features
✓ Minimal file format overhead
✓ 50-80% smaller than old format **bruh** even uncompressed
✓ Optional deflate compression
✓ Support for RGB and RGBA
✓ Fast encoding/decoding
✓ Cross-platform

## Known Issues
1. Slower than PNG for photo-realistic images (use PNG for photos)
2. Better for graphics/pixel art
