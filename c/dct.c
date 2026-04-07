/* SPDX-License-Identifier: GPL-3.0-or-later
 * DCT/IDCT 8×8. SSE2/NEON when available; scalar fallback.
 * Each implementation is compiled only when its target is active,
 * eliminating dead-code warnings on platforms with SIMD support.
 */
#include "dct.h"
#include <math.h>
#include <string.h>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

/* Precomputed cos((2*x+1)*u*pi/16) for x,u in 0..7. Row u, col x.
 * Shared by all implementations (SSE2/NEON load from this table). */
static const float COS_TABLE[8][8] = {
    { 1.000000f, 1.000000f, 1.000000f, 1.000000f, 1.000000f, 1.000000f, 1.000000f, 1.000000f },
    { 0.980785f, 0.831470f, 0.555570f, 0.195090f,-0.195090f,-0.555570f,-0.831470f,-0.980785f },
    { 0.923880f, 0.382683f,-0.382683f,-0.923880f,-0.923880f,-0.382683f, 0.382683f, 0.923880f },
    { 0.831470f,-0.195090f,-0.980785f,-0.555570f, 0.555570f, 0.980785f, 0.195090f,-0.831470f },
    { 0.707107f,-0.707107f,-0.707107f, 0.707107f, 0.707107f,-0.707107f,-0.707107f, 0.707107f },
    { 0.555570f,-0.980785f, 0.195090f, 0.831470f,-0.831470f,-0.195090f, 0.980785f,-0.555570f },
    { 0.382683f,-0.923880f, 0.923880f,-0.382683f,-0.382683f, 0.923880f,-0.923880f, 0.382683f },
    { 0.195090f,-0.555570f, 0.831470f,-0.980785f, 0.980785f,-0.831470f, 0.555570f,-0.195090f },
};

#define INV_SQRT2 0.70710678118654752440f

/* ------------------------------------------------------------------ */
/* AVX2 implementation                                                  */
/* ------------------------------------------------------------------ */
#if defined(__AVX2__)
#include <immintrin.h>

static inline float hsum_ps_avx(__m256 v)
{
    __m128 lo = _mm256_castps256_ps128(v);
    __m128 hi = _mm256_extractf128_ps(v, 1);
    __m128 s = _mm_add_ps(lo, hi);
    __m128 t = _mm_add_ps(s, _mm_movehl_ps(s, s));
    t = _mm_add_ss(t, _mm_shuffle_ps(t, t, 0x55));
    return _mm_cvtss_f32(t);
}

static void dct_1d_avx2(const float *in, float *out)
{
    __m256 vin = _mm256_loadu_ps(in);
    for (int u = 0; u < 8; u++) {
        __m256 vc = _mm256_loadu_ps(&COS_TABLE[u][0]);
        float total = hsum_ps_avx(_mm256_mul_ps(vin, vc));
        out[u] = ((u == 0) ? 0.5f * INV_SQRT2 : 0.5f) * total;
    }
}

static void idct_1d_avx2(const float *in, float *out)
{
    float sc[8];
    sc[0] = INV_SQRT2 * in[0];
    for (int u = 1; u < 8; u++) sc[u] = in[u];
    __m256 vin = _mm256_loadu_ps(sc);
    for (int x = 0; x < 8; x++) {
        __m256 vc = _mm256_setr_ps(
            COS_TABLE[0][x], COS_TABLE[1][x], COS_TABLE[2][x], COS_TABLE[3][x],
            COS_TABLE[4][x], COS_TABLE[5][x], COS_TABLE[6][x], COS_TABLE[7][x]
        );
        out[x] = 0.5f * hsum_ps_avx(_mm256_mul_ps(vin, vc));
    }
}

static void dct_block_avx2(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = (float)block[y * 8 + x];
        dct_1d_avx2(row, col);
        for (int u = 0; u < 8; u++) tmp[y * 8 + u] = col[u];
    }
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = tmp[v * 8 + u];
        dct_1d_avx2(col, row);
        for (int v = 0; v < 8; v++) block[v * 8 + u] = (int16_t)lroundf(row[v]);
    }
}

static void idct_block_avx2(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = (float)block[v * 8 + u];
        idct_1d_avx2(col, row);
        for (int v = 0; v < 8; v++) tmp[v * 8 + u] = row[v];
    }
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = tmp[y * 8 + x];
        idct_1d_avx2(row, col);
        for (int x = 0; x < 8; x++) block[y * 8 + x] = (int16_t)lroundf(col[x]);
    }
}

/* ------------------------------------------------------------------ */
/* SSE2 implementation                                                  */
/* ------------------------------------------------------------------ */
#elif defined(__SSE2__)
#include <emmintrin.h>

static inline float hsum_ps_sse2(__m128 v)
{
    __m128 t = _mm_add_ps(v, _mm_movehl_ps(v, v));
    t = _mm_add_ss(t, _mm_shuffle_ps(t, t, 0x55));
    return _mm_cvtss_f32(t);
}

static void dct_1d_sse2(const float *in, float *out)
{
    for (int u = 0; u < 8; u++) {
        __m128 sum = _mm_setzero_ps();
        for (int x = 0; x < 8; x += 4) {
            __m128 a = _mm_loadu_ps(&in[x]);
            __m128 b = _mm_loadu_ps(&COS_TABLE[u][x]);
            sum = _mm_add_ps(sum, _mm_mul_ps(a, b));
        }
        float total = hsum_ps_sse2(sum);
        out[u] = ((u == 0) ? 0.5f * INV_SQRT2 : 0.5f) * total;
    }
}

static void idct_1d_sse2(const float *in, float *out)
{
    const float sc0 = INV_SQRT2 * in[0];
    __m128 in0 = _mm_setr_ps(sc0, in[1], in[2], in[3]);
    __m128 in1 = _mm_setr_ps(in[4], in[5], in[6], in[7]);
    for (int x = 0; x < 8; x++) {
        __m128 sum = _mm_setzero_ps();
        __m128 c0 = _mm_setr_ps(COS_TABLE[0][x], COS_TABLE[1][x], COS_TABLE[2][x], COS_TABLE[3][x]);
        __m128 c1 = _mm_setr_ps(COS_TABLE[4][x], COS_TABLE[5][x], COS_TABLE[6][x], COS_TABLE[7][x]);
        sum = _mm_add_ps(sum, _mm_mul_ps(in0, c0));
        sum = _mm_add_ps(sum, _mm_mul_ps(in1, c1));
        out[x] = 0.5f * hsum_ps_sse2(sum);
    }
}

static void dct_block_sse2(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = (float)block[y * 8 + x];
        dct_1d_sse2(row, col);
        for (int u = 0; u < 8; u++) tmp[y * 8 + u] = col[u];
    }
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = tmp[v * 8 + u];
        dct_1d_sse2(col, row);
        for (int v = 0; v < 8; v++) block[v * 8 + u] = (int16_t)lroundf(row[v]);
    }
}

static void idct_block_sse2(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = (float)block[v * 8 + u];
        idct_1d_sse2(col, row);
        for (int v = 0; v < 8; v++) tmp[v * 8 + u] = row[v];
    }
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = tmp[y * 8 + x];
        idct_1d_sse2(row, col);
        for (int x = 0; x < 8; x++) block[y * 8 + x] = (int16_t)lroundf(col[x]);
    }
}

/* ------------------------------------------------------------------ */
/* NEON implementation                                                  */
/* ------------------------------------------------------------------ */
#elif defined(__ARM_NEON) || defined(__aarch64__)
#include <arm_neon.h>

static void dct_1d_neon(const float *in, float *out)
{
    for (int u = 0; u < 8; u++) {
        float32x4_t sum = vdupq_n_f32(0.f);
        sum = vmlaq_f32(sum, vld1q_f32(&in[0]), vld1q_f32(&COS_TABLE[u][0]));
        sum = vmlaq_f32(sum, vld1q_f32(&in[4]), vld1q_f32(&COS_TABLE[u][4]));
        float s = vgetq_lane_f32(sum, 0) + vgetq_lane_f32(sum, 1)
                + vgetq_lane_f32(sum, 2) + vgetq_lane_f32(sum, 3);
        out[u] = ((u == 0) ? 0.5f * INV_SQRT2 : 0.5f) * s;
    }
}

static void idct_1d_neon(const float *in, float *out)
{
    float sc[8];
    sc[0] = INV_SQRT2 * in[0];
    for (int u = 1; u < 8; u++) sc[u] = in[u];
    for (int x = 0; x < 8; x++) {
        float s = 0.f;
        for (int u = 0; u < 8; u++) s += sc[u] * COS_TABLE[u][x];
        out[x] = 0.5f * s;
    }
}

static void dct_block_neon(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = (float)block[y * 8 + x];
        dct_1d_neon(row, col);
        for (int u = 0; u < 8; u++) tmp[y * 8 + u] = col[u];
    }
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = tmp[v * 8 + u];
        dct_1d_neon(col, row);
        for (int v = 0; v < 8; v++) block[v * 8 + u] = (int16_t)lroundf(row[v]);
    }
}

static void idct_block_neon(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = (float)block[v * 8 + u];
        idct_1d_neon(col, row);
        for (int v = 0; v < 8; v++) tmp[v * 8 + u] = row[v];
    }
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = tmp[y * 8 + x];
        idct_1d_neon(row, col);
        for (int x = 0; x < 8; x++) block[y * 8 + x] = (int16_t)lroundf(col[x]);
    }
}

/* ------------------------------------------------------------------ */
/* Scalar fallback — compiled only when no SIMD is available           */
/* ------------------------------------------------------------------ */
#else

static void dct_1d(const float *in, float *out)
{
    for (int u = 0; u < 8; u++) {
        float sum = 0.f;
        for (int x = 0; x < 8; x++)
            sum += in[x] * COS_TABLE[u][x];
        out[u] = ((u == 0) ? 0.5f * INV_SQRT2 : 0.5f) * sum;
    }
}

static void idct_1d(const float *in, float *out)
{
    float sc[8];
    sc[0] = INV_SQRT2 * in[0];
    for (int u = 1; u < 8; u++) sc[u] = in[u];
    for (int x = 0; x < 8; x++) {
        float sum = 0.f;
        for (int u = 0; u < 8; u++)
            sum += sc[u] * COS_TABLE[u][x];
        out[x] = 0.5f * sum;
    }
}

static void dct_block_scalar(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = (float)block[y * 8 + x];
        dct_1d(row, col);
        for (int u = 0; u < 8; u++) tmp[y * 8 + u] = col[u];
    }
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = tmp[v * 8 + u];
        dct_1d(col, row);
        for (int v = 0; v < 8; v++) block[v * 8 + u] = (int16_t)lroundf(row[v]);
    }
}

static void idct_block_scalar(int16_t *block)
{
    float tmp[64], row[8], col[8];
    for (int u = 0; u < 8; u++) {
        for (int v = 0; v < 8; v++) col[v] = (float)block[v * 8 + u];
        idct_1d(col, row);
        for (int v = 0; v < 8; v++) tmp[v * 8 + u] = row[v];
    }
    for (int y = 0; y < 8; y++) {
        for (int x = 0; x < 8; x++) row[x] = tmp[y * 8 + x];
        idct_1d(row, col);
        for (int x = 0; x < 8; x++) block[y * 8 + x] = (int16_t)lroundf(col[x]);
    }
}

#endif /* SIMD dispatch */

/* ------------------------------------------------------------------ */
/* Public entry points — dispatch to the active implementation         */
/* ------------------------------------------------------------------ */

void bitgrain_dct_block(int16_t *block)
{
#if defined(__ARM_NEON) || defined(__aarch64__)
    dct_block_neon(block);
#elif defined(__AVX2__)
    dct_block_avx2(block);
#elif defined(__SSE2__)
    dct_block_sse2(block);
#else
    dct_block_scalar(block);
#endif
}

void bitgrain_idct_block(int16_t *block)
{
#if defined(__ARM_NEON) || defined(__aarch64__)
    idct_block_neon(block);
#elif defined(__AVX2__)
    idct_block_avx2(block);
#elif defined(__SSE2__)
    idct_block_sse2(block);
#else
    idct_block_scalar(block);
#endif
}
