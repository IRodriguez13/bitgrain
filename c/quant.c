/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "quant.h"
#include <stdint.h>

/* Scalar fallback: block[i] /= table[i] */
static void quantize_block_scalar(int16_t *block, const int16_t *table)
{
    for (int i = 0; i < 64; i++)
        block[i] = block[i] / table[i];
}

#if defined(__SSE2__)
#include <emmintrin.h>

/* SSE2: 4 int16 at a time via float division (no native int16 div in SSE2). */
static void quantize_block_sse2(int16_t *block, const int16_t *table)
{
    for (int i = 0; i < 64; i += 4) {
        __m128i b = _mm_loadl_epi64((const __m128i *)&block[i]);
        __m128i t = _mm_loadl_epi64((const __m128i *)&table[i]);
        __m128i bl = _mm_unpacklo_epi16(b, _mm_cmplt_epi16(b, _mm_setzero_si128()));
        __m128i tl = _mm_unpacklo_epi16(t, _mm_setzero_si128());
        __m128 fl = _mm_cvtepi32_ps(bl);
        __m128 ft = _mm_cvtepi32_ps(tl);
        __m128 fd = _mm_div_ps(fl, ft);
        __m128i rl = _mm_cvtps_epi32(fd);
        __m128i bh = _mm_unpackhi_epi16(b, _mm_cmplt_epi16(b, _mm_setzero_si128()));
        __m128i th = _mm_unpackhi_epi16(t, _mm_setzero_si128());
        __m128 fh = _mm_cvtepi32_ps(bh);
        __m128 fth = _mm_cvtepi32_ps(th);
        __m128 fdh = _mm_div_ps(fh, fth);
        __m128i rh = _mm_cvtps_epi32(fdh);
        __m128i r = _mm_packs_epi32(rl, rh);
        _mm_storel_epi64((__m128i *)&block[i], r);
    }
}
#endif

#if defined(__ARM_NEON) || defined(__aarch64__)
#include <arm_neon.h>

/* NEON: 4 int16 at a time via float. */
static void quantize_block_neon(int16_t *block, const int16_t *table)
{
    for (int i = 0; i < 64; i += 4) {
        int16x4_t b = vld1_s16(&block[i]);
        int16x4_t t = vld1_s16(&table[i]);
        float32x4_t fb = vcvtq_f32_s32(vmovl_s16(b));
        float32x4_t ft = vcvtq_f32_s32(vmovl_s16(t));
        float32x4_t fd = vdivq_f32(fb, ft);
        int16x4_t r = vmovn_s32(vcvtq_s32_f32(fd));
        vst1_s16(&block[i], r);
    }
}
#endif

void quantize_block(int16_t *block, const int16_t *table)
{
#if defined(__ARM_NEON) || defined(__aarch64__)
    quantize_block_neon(block, table);
#elif defined(__SSE2__)
    quantize_block_sse2(block, table);
#else
    quantize_block_scalar(block, table);
#endif
}
