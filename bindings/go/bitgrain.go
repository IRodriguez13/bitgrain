// Package bitgrain provides Go bindings for the Bitgrain image codec (.bg encode/decode).
//
// Build the library first (from repo root): make bitgrain
// Then build with cgo, e.g.:
//
//	CGO_ENABLED=1 go build
//	# Or with explicit paths:
//	CGO_CFLAGS="-I/path/to/bitgrain/includes" CGO_LDFLAGS="-L/path/to/bitgrain/rust/target/release -lbitgrain -lpthread -ldl -lm" go build
package bitgrain

/*
#cgo CFLAGS: -I${SRCDIR}/../../includes
#cgo LDFLAGS: -L${SRCDIR}/../../rust/target/release -lbitgrain -lpthread -ldl -lm -lwebp

#include "encoder.h"
*/
import "C"
import (
	"unsafe"
)

// EncodeGrayscale encodes 8-bit grayscale to .bg. Quality 1–100, 0 = 85. Returns nil on error.
func EncodeGrayscale(image []byte, width, height uint32, quality uint8) []byte {
	if len(image) < int(width)*int(height) || width == 0 || height == 0 {
		return nil
	}
	cap := width * height * 2
	if cap < 1024 {
		cap = 1024
	}
	out := make([]byte, cap)
	var outLen C.int32_t
	ok := C.bitgrain_encode_grayscale(
		(*C.uint8_t)(unsafe.Pointer(&image[0])),
		C.uint32_t(width), C.uint32_t(height),
		(*C.uint8_t)(unsafe.Pointer(&out[0])), C.uint32_t(cap), &outLen,
		C.uint8_t(quality),
	)
	if ok != 0 {
		return nil
	}
	return out[:outLen]
}

// EncodeRGB encodes RGB (24 bpp, R G B per pixel) to .bg. Quality 1–100, 0 = 85.
func EncodeRGB(image []byte, width, height uint32, quality uint8) []byte {
	if len(image) < int(width)*int(height)*3 || width == 0 || height == 0 {
		return nil
	}
	cap := width * height * 3 * 2
	if cap < 1024 {
		cap = 1024
	}
	out := make([]byte, cap)
	var outLen C.int32_t
	ok := C.bitgrain_encode_rgb(
		(*C.uint8_t)(unsafe.Pointer(&image[0])),
		C.uint32_t(width), C.uint32_t(height),
		(*C.uint8_t)(unsafe.Pointer(&out[0])), C.uint32_t(cap), &outLen,
		C.uint8_t(quality),
	)
	if ok != 0 {
		return nil
	}
	return out[:outLen]
}

// EncodeRGBA encodes RGBA (32 bpp) to .bg. Quality 1–100, 0 = 85.
func EncodeRGBA(image []byte, width, height uint32, quality uint8) []byte {
	if len(image) < int(width)*int(height)*4 || width == 0 || height == 0 {
		return nil
	}
	cap := width * height * 4 * 2
	if cap < 1024 {
		cap = 1024
	}
	out := make([]byte, cap)
	var outLen C.int32_t
	ok := C.bitgrain_encode_rgba(
		(*C.uint8_t)(unsafe.Pointer(&image[0])),
		C.uint32_t(width), C.uint32_t(height),
		(*C.uint8_t)(unsafe.Pointer(&out[0])), C.uint32_t(cap), &outLen,
		C.uint8_t(quality),
	)
	if ok != 0 {
		return nil
	}
	return out[:outLen]
}

// DecodeResult holds decoded pixel data and dimensions.
type DecodeResult struct {
	Pixels  []byte
	Width   uint32
	Height  uint32
	Channels uint32 // 1=gray, 3=RGB, 4=RGBA
}

// Decode decodes a .bg stream. Returns nil on error.
func Decode(data []byte) *DecodeResult {
	if len(data) < 12 {
		return nil
	}
	// Header: magic(2) + version(1) + width(4) + height(4) + quality(1)
	w := uint32(data[2]) | uint32(data[3])<<8 | uint32(data[4])<<16 | uint32(data[5])<<24
	h := uint32(data[6]) | uint32(data[7])<<8 | uint32(data[8])<<16 | uint32(data[9])<<24
	cap := w * h * 4
	if cap == 0 {
		return nil
	}
	out := make([]byte, cap)
	var outW, outH, outCh C.uint32_t
	ok := C.bitgrain_decode(
		(*C.uint8_t)(unsafe.Pointer(&data[0])), C.int32_t(len(data)),
		(*C.uint8_t)(unsafe.Pointer(&out[0])), C.uint32_t(cap),
		&outW, &outH, &outCh,
	)
	if ok != 0 {
		return nil
	}
	ch := uint32(outCh)
	return &DecodeResult{
		Pixels:   out[:outW*outH*outCh],
		Width:    uint32(outW),
		Height:   uint32(outH),
		Channels: ch,
	}
}
