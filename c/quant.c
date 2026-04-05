/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "quant.h"
#include <stdint.h>

#if defined(__SSE2__)
#include <emmintrin.h>

/* SSE2: process 8 int16 per iteration (full 128-bit register).
 * FIX: previous code used _mm_loadl_epi64 (4 i16) then _mm_unpackhi_epi16
 * on the same 64-bit load, producing garbage in the high half.
 * Now we load 8 i16 at once and split into two groups of 4 for float division. */
static void quantize_block_sse2(int16_t *block, const int16_t *table)
{
    __m128i zero = _mm_setzero_si128();
    for (int i = 0; i < 64; i += 8) {
        __m128i b = _mm_loadu_si128((const __m128i *)&block[i]);
        __m128i t = _mm_loadu_si128((const __m128i *)&table[i]);

        __m128i b_sign = _mm_cmplt_epi16(b, zero);
        __m128i bl = _mm_unpacklo_epi16(b, b_sign);
        __m128i tl = _mm_unpacklo_epi16(t, zero);
        __m128i rl = _mm_cvtps_epi32(_mm_div_ps(_mm_cvtepi32_ps(bl), _mm_cvtepi32_ps(tl)));

        __m128i bh = _mm_unpackhi_epi16(b, b_sign);
        __m128i th = _mm_unpackhi_epi16(t, zero);
        __m128i rh = _mm_cvtps_epi32(_mm_div_ps(_mm_cvtepi32_ps(bh), _mm_cvtepi32_ps(th)));

        _mm_storeu_si128((__m128i *)&block[i], _mm_packs_epi32(rl, rh));
    }
}

void quantize_block(int16_t *block, const int16_t *table)
{
    quantize_block_sse2(block, table);
}

#elif defined(__ARM_NEON) || defined(__aarch64__)
#include <arm_neon.h>

/* NEON: 4 int16 at a time via float. */
static void quantize_block_neon(int16_t *block, const int16_t *table)
{
    for (int i = 0; i < 64; i += 4) {
        int16x4_t b = vld1_s16(&block[i]);
        int16x4_t t = vld1_s16(&table[i]);
        float32x4_t fb = vcvtq_f32_s32(vmovl_s16(b));
        float32x4_t ft = vcvtq_f32_s32(vmovl_s16(t));
        int16x4_t r = vmovn_s32(vcvtq_s32_f32(vdivq_f32(fb, ft)));
        vst1_s16(&block[i], r);
    }
}

void quantize_block(int16_t *block, const int16_t *table)
{
    quantize_block_neon(block, table);
}

#else

/* Scalar fallback — compiled only when no SIMD is available. */
void quantize_block(int16_t *block, const int16_t *table)
{
    for (int i = 0; i < 64; i++)
        block[i] = block[i] / table[i];
}

#endif
