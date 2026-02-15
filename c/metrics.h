#ifndef BITGRAIN_METRICS_H
#define BITGRAIN_METRICS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * PSNR (Peak Signal-to-Noise Ratio) in dB.
 * orig and recon: same size, width*height*channels (1 or 3).
 * Returns PSNR or 0 if MSE is 0 (identical images).
 */
double bitgrain_psnr(const uint8_t *orig, const uint8_t *recon,
                     uint32_t width, uint32_t height, uint32_t channels);

/*
 * SSIM (Structural Similarity) in [0, 1]. Higher = closer to original.
 * Same layout as PSNR. Uses a single global window (fast).
 */
double bitgrain_ssim(const uint8_t *orig, const uint8_t *recon,
                     uint32_t width, uint32_t height, uint32_t channels);

#ifdef __cplusplus
}
#endif

#endif
