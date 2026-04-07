/* SPDX-License-Identifier: GPL-3.0-or-later */
#include "quant.h"
#include <stdint.h>

#if defined(__AVX2__)
#include <immintrin.h>

/* AVX2: process 16 int16 per iteration. */
static void quantize_block_avx2(int16_t *block, const int16_t *table)
{
    for (int i = 0; i < 64; i += 16) {
        __m256i b = _mm256_loadu_si256((const __m256i *)&block[i]);
        __m256i t = _mm256_loadu_si256((const __m256i *)&table[i]);

        __m256i bl = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(b));
        __m256i tl = _mm256_cvtepi16_epi32(_mm256_castsi256_si128(t));
        __m256i rl = _mm256_cvtps_epi32(
            _mm256_div_ps(_mm256_cvtepi32_ps(bl), _mm256_cvtepi32_ps(tl)));

        __m256i bh = _mm256_cvtepi16_epi32(_mm256_extracti128_si256(b, 1));
        __m256i th = _mm256_cvtepi16_epi32(_mm256_extracti128_si256(t, 1));
        __m256i rh = _mm256_cvtps_epi32(
            _mm256_div_ps(_mm256_cvtepi32_ps(bh), _mm256_cvtepi32_ps(th)));

        __m128i rl_lo = _mm256_castsi256_si128(rl);
        __m128i rl_hi = _mm256_extracti128_si256(rl, 1);
        __m128i rh_lo = _mm256_castsi256_si128(rh);
        __m128i rh_hi = _mm256_extracti128_si256(rh, 1);
        __m128i out_lo = _mm_packs_epi32(rl_lo, rl_hi);
        __m128i out_hi = _mm_packs_epi32(rh_lo, rh_hi);
        _mm_storeu_si128((__m128i *)&block[i], out_lo);
        _mm_storeu_si128((__m128i *)&block[i + 8], out_hi);
    }
}

void quantize_block(int16_t *block, const int16_t *table)
{
    quantize_block_avx2(block, table);
}

#elif defined(__SSE2__)
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
