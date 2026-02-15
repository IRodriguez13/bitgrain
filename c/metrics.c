/* PSNR and SSIM for 8-bit images. No extra deps. */

#include "metrics.h"
#include <math.h>
#include <string.h>

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

double bitgrain_psnr(const uint8_t *orig, const uint8_t *recon,
                     uint32_t width, uint32_t height, uint32_t channels)
{
    uint64_t n = (uint64_t)width * height * channels;
    if (n == 0) return 0.0;

    uint64_t sum_sq = 0;
    for (uint64_t i = 0; i < n; i++) {
        int d = (int)orig[i] - (int)recon[i];
        sum_sq += (uint64_t)(d * d);
    }
    double mse = (double)sum_sq / (double)n;
    if (mse <= 0.0) return 99.0; /* identical */
    return 10.0 * log10((255.0 * 255.0) / mse);
}

/* SSIM constants (standard). */
#define C1 6.5025    /* (0.01*255)^2 */
#define C2 58.5225   /* (0.03*255)^2 */

double bitgrain_ssim(const uint8_t *orig, const uint8_t *recon,
                     uint32_t width, uint32_t height, uint32_t channels)
{
    uint64_t n = (uint64_t)width * height * channels;
    if (n == 0) return 0.0;

    double mu_x = 0.0, mu_y = 0.0;
    for (uint64_t i = 0; i < n; i++) {
        mu_x += orig[i];
        mu_y += recon[i];
    }
    mu_x /= (double)n;
    mu_y /= (double)n;

    double sigma_x2 = 0.0, sigma_y2 = 0.0, sigma_xy = 0.0;
    for (uint64_t i = 0; i < n; i++) {
        double dx = (double)orig[i] - mu_x;
        double dy = (double)recon[i] - mu_y;
        sigma_x2 += dx * dx;
        sigma_y2 += dy * dy;
        sigma_xy += dx * dy;
    }
    sigma_x2 /= (double)n;
    sigma_y2 /= (double)n;
    sigma_xy /= (double)n;

    double l = (2.0 * mu_x * mu_y + C1) / (mu_x * mu_x + mu_y * mu_y + C1);
    double sig_x = sqrt(sigma_x2), sig_y = sqrt(sigma_y2);
    double c = (2.0 * sig_x * sig_y + C2) / (sigma_x2 + sigma_y2 + C2);
    double s = (sigma_xy + C2 / 2.0) / (sig_x * sig_y + C2 / 2.0);

    return l * c * s;
}
