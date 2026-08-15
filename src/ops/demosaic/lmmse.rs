use crate::opbasics::*;

/// Zhang-Wu directional LMMSE demosaicing.
///
/// Based on Pascal Getreuer's reference implementation of:
///
///   L. Zhang and X. Wu,
///   "Color demosaicking via directional linear minimum
///    mean square-error estimation,"
///   IEEE Transactions on Image Processing, 14(12), 2167-2178, 2005.
///
/// Getreuer's implementation:
///
///   P. Getreuer, "Zhang-Wu Directional LMMSE Image Demosaicking",
///   Image Processing On Line, 1 (2011), 117-126.
///
/// This implementation uses the formulation described in the
/// Zhang/Wu paper (Getreuer's UseZhangCodeEst = 0).
///
/// Input:
///   buf.colors == 1, containing one normalized CFA sample per pixel.
///
/// Output:
///   4 channels: R, G, B, 0.
///
/// Only standard 2x2 RGB Bayer CFAs are supported.
pub fn lmmse(cfa: &CFA, buf: &OpBuffer) -> OpBuffer {
    assert!(
        cfa.width == 2 && cfa.height == 2,
        "Zhang-Wu LMMSE requires a 2x2 Bayer CFA"
    );

    assert!(
        buf.colors == 1,
        "Zhang-Wu LMMSE requires a single-channel CFA buffer"
    );

    let width = buf.width;
    let height = buf.height;
    let npix = width * height;

    let mut out = OpBuffer::new(
        width,
        height,
        4,
        buf.monochrome,
    );

    if npix == 0 {
        return out;
    }

    /*
     * The reference algorithm assumes that both dimensions have
     * at least two pixels. Handle degenerate images separately.
     */
    if width == 1 || height == 1 {
        out.mutate_lines(&(|line: &mut [f32], row| {
            for x in 0..width {
                let v = buf.data[row * width + x];
                let o = x * 4;

                line[o] = v;
                line[o + 1] = v;
                line[o + 2] = v;
                line[o + 3] = 0.0;
            }
        }));

        return out;
    }

    /*
     * ---------------------------------------------------------------
     * Determine the Bayer pattern.
     * ---------------------------------------------------------------
     *
     * The four standard patterns are:
     *
     *     RGGB    red=(0,0)
     *     GRBG    red=(0,1)
     *     GBRG    red=(1,0)
     *     BGGR    red=(1,1)
     */

    let mut red_x = None;
    let mut red_y = None;

    let mut nr = 0;
    let mut ng = 0;
    let mut nb = 0;

    for y in 0..2 {
        for x in 0..2 {
            match cfa.color_at(y, x) {
                0 => {
                    nr += 1;
                    red_x = Some(x);
                    red_y = Some(y);
                }
                1 => {
                    ng += 1;
                }
                2 => {
                    nb += 1;
                }
                _ => {
                    panic!(
                        "Zhang-Wu LMMSE requires an RGB Bayer CFA"
                    );
                }
            }
        }
    }

    assert!(
        nr == 1 && ng == 2 && nb == 1,
        "Zhang-Wu LMMSE requires a standard RGB Bayer CFA"
    );

    let red_x = red_x.unwrap();
    let red_y = red_y.unwrap();

    /*
     * This is exactly Getreuer's:
     *
     *     Green = 1 - ((RedX + RedY) & 1);
     */
    let green = 1 - ((red_x + red_y) & 1);

    /*
     * ---------------------------------------------------------------
     * Constants from Getreuer.
     * ---------------------------------------------------------------
     */

    // Window size for estimating LMMSE statistics.
    const M: isize = 4;

    /*
     * Getreuer's reference implementation uses 0.1/(255*255).
     *
     * This is the normalized-domain equivalent of adding 0.1 to a
     * variance expressed in 8-bit (0..255) units. Variance scales
     * with the square of the signal scale, so the conversion from
     * 8-bit variance to normalized variance requires division by
     * 255².
     *
     * OpBuffer values are already normalized to [0,1], so use this
     * normalized value directly.
     */
    const DIV_EPSILON: f32 = 0.1 / (255.0 * 255.0);

    /*
     * Interpolation filter:
     *
     *     {-0.25, 0.5, 0.5, 0.5, -0.25}
     *
     * with delay -2.
     */
    const INTERP: [f32; 5] = [
        -0.25,
        0.50,
        0.50,
        0.50,
        -0.25,
    ];

    /*
     * Approximately Gaussian smoothing filter from Getreuer.
     *
     * with delay -4.
     */
    const SMOOTH: [f32; 9] = [
        0.03125,
        0.0703125,
        0.1171875,
        0.1796875,
        0.203125,
        0.1796875,
        0.1171875,
        0.0703125,
        0.03125,
    ];

    /*
     * ---------------------------------------------------------------
     * Whole-sample symmetric boundary extension.
     * ---------------------------------------------------------------
     *
     * This corresponds to Getreuer's "symw" boundary extension.
     */
    #[inline(always)]
    fn symw(index: isize, n: usize) -> usize {
        debug_assert!(n >= 2);

        let n = n as isize;
        let mut i = index;

        while i < 0 || i >= n {
            if i < 0 {
                i = -i;
            } else {
                i = 2 * (n - 1) - i;
            }
        }

        i as usize
    }

    /*
     * ---------------------------------------------------------------
     * 1D convolution.
     * ---------------------------------------------------------------
     *
     * Equivalent to Getreuer's Conv1D with:
     *
     *     n - Filter.Delay - k
     *
     * as the source coordinate.
     *
     * For INTERP, Delay = -2.
     * For SMOOTH, Delay = -4.
     */
    #[inline]
    fn conv_horizontal(
        src: &[f32],
        dst: &mut [f32],
        width: usize,
        height: usize,
        coeff: &[f32],
        delay: isize,
    ) {
        for y in 0..height {
            let base = y * width;

            for x in 0..width {
                let mut sum = 0.0f32;

                for k in 0..coeff.len() {
                    let p =
                        x as isize
                        - delay
                        - k as isize;

                    let p = symw(p, width);

                    sum += coeff[k] * src[base + p];
                }

                dst[base + x] = sum;
            }
        }
    }

    #[inline]
    fn conv_vertical(
        src: &[f32],
        dst: &mut [f32],
        width: usize,
        height: usize,
        coeff: &[f32],
        delay: isize,
    ) {
        for y in 0..height {
            for x in 0..width {
                let mut sum = 0.0f32;

                for k in 0..coeff.len() {
                    let p =
                        y as isize
                        - delay
                        - k as isize;

                    let p = symw(p, height);

                    sum += coeff[k] * src[p * width + x];
                }

                dst[y * width + x] = sum;
            }
        }
    }

    /*
     * ---------------------------------------------------------------
     * Workspace.
     * ---------------------------------------------------------------
     *
     * Getreuer uses:
     *
     *     FilteredH
     *     FilteredV
     *     DiffH / DiffGR
     *     DiffV / DiffGB
     */
    let mut filtered_h = vec![0.0f32; npix];
    let mut filtered_v = vec![0.0f32; npix];

    let mut diff_h = vec![0.0f32; npix];
    let mut diff_v = vec![0.0f32; npix];

    /*
     * ---------------------------------------------------------------
     * Horizontal and vertical interpolation.
     * ---------------------------------------------------------------
     */
    conv_horizontal(
        &buf.data,
        &mut filtered_h,
        width,
        height,
        &INTERP,
        -2,
    );

    conv_vertical(
        &buf.data,
        &mut filtered_v,
        width,
        height,
        &INTERP,
        -2,
    );

    /*
     * ---------------------------------------------------------------
     * Local noise estimation for LMMSE.
     * ---------------------------------------------------------------
     */
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;

            if ((x + y) & 1) == green {
                diff_h[i] =
                    buf.data[i] - filtered_h[i];

                diff_v[i] =
                    buf.data[i] - filtered_v[i];
            } else {
                diff_h[i] =
                    filtered_h[i] - buf.data[i];

                diff_v[i] =
                    filtered_v[i] - buf.data[i];
            }
        }
    }

    /*
     * ---------------------------------------------------------------
     * Smooth the difference signals.
     *
     * The filtered buffers are deliberately reused as workspace,
     * exactly as in the reference implementation.
     * ---------------------------------------------------------------
     */
    conv_horizontal(
        &diff_h,
        &mut filtered_h,
        width,
        height,
        &SMOOTH,
        -4,
    );

    conv_vertical(
        &diff_v,
        &mut filtered_v,
        width,
        height,
        &SMOOTH,
        -4,
    );

    /*
     * ---------------------------------------------------------------
     * LMMSE interpolation of green.
     * ---------------------------------------------------------------
     */
    let mut green_channel = vec![0.0f32; npix];

    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;

            /*
             * Green samples are already known.
             */
            if ((x + y) & 1) == green {
                green_channel[i] = buf.data[i];
                continue;
            }

            /*
             * -------------------------------------------------------
             * Horizontal statistics.
             * -------------------------------------------------------
             *
             * The statistical window is clipped at the image
             * boundary. Unlike the original C implementation, the
             * normalization below uses the actual number of samples
             * in the clipped window.
             */
            let m0 =
                if x >= M as usize {
                    -M
                } else {
                    -(x as isize)
                };

            let m1 =
                if x < width - M as usize {
                    M
                } else {
                    width as isize
                        - x as isize
                        - 1
                };

            let n_h = (m1 - m0 + 1) as f32;

            let mut mom1 = 0.0f32;
            let mut ph = 0.0f32;
            let mut rh = 0.0f32;

            for m in m0..=m1 {
                let q =
                    (x as isize + m) as usize
                    + y * width;

                let temp = filtered_h[q];

                mom1 += temp;
                ph += temp * temp;

                let temp =
                    temp - diff_h[q];

                rh += temp * temp;
            }

            /*
             * Mean of the smoothed signal.
             */
            let mh = mom1 / n_h;

            /*
             * Sample variance of the smoothed signal.
             *
             * For n samples:
             *
             *     var = sum(x²)/(n-1)
             *           - sum(x)²/(n(n-1))
             *
             * This reduces to Getreuer's:
             *
             *     ph/(2M)
             *       - mom1²/(2M(2M+1))
             *
             * when n = 2M+1.
             */
            let mut ph =
                if n_h > 1.0 {
                    ph / (n_h - 1.0)
                        - mom1 * mom1
                            / (n_h * (n_h - 1.0))
                } else {
                    0.0
                };

            /*
             * Guard against tiny negative values caused by floating
             * point roundoff.
             */
            if ph < 0.0 {
                ph = 0.0;
            }

            /*
             * Noise variance.
             */
            let rh =
                rh / n_h + DIV_EPSILON;

            /*
             * Horizontal LMMSE estimate.
             */
            let h =
                mh
                + (ph / (ph + rh))
                    * (diff_h[i] - mh);

            /*
             * Estimated variance / accuracy.
             */
            let h_error =
                ph
                - (ph / (ph + rh)) * ph
                + DIV_EPSILON;

            /*
             * -------------------------------------------------------
             * Vertical statistics.
             * -------------------------------------------------------
             */
            let m0 =
                if y >= M as usize {
                    -M
                } else {
                    -(y as isize)
                };

            let m1 =
                if y < height - M as usize {
                    M
                } else {
                    height as isize
                        - y as isize
                        - 1
                };

            let n_v = (m1 - m0 + 1) as f32;

            let mut mom1 = 0.0f32;
            let mut pv = 0.0f32;
            let mut rv = 0.0f32;

            for m in m0..=m1 {
                let q =
                    x
                    + (y as isize + m) as usize
                        * width;

                let temp = filtered_v[q];

                mom1 += temp;
                pv += temp * temp;

                let temp =
                    temp - diff_v[q];

                rv += temp * temp;
            }

            /*
             * Mean of the smoothed signal.
             */
            let mv = mom1 / n_v;

            /*
             * Sample variance of the smoothed signal.
             */
            let mut pv =
                if n_v > 1.0 {
                    pv / (n_v - 1.0)
                        - mom1 * mom1
                            / (n_v * (n_v - 1.0))
                } else {
                    0.0
                };

            /*
             * Guard against tiny negative values caused by floating
             * point roundoff.
             */
            if pv < 0.0 {
                pv = 0.0;
            }

            /*
             * Noise variance.
             */
            let rv =
                rv / n_v + DIV_EPSILON;

            /*
             * Vertical LMMSE estimate.
             */
            let v =
                mv
                + (pv / (pv + rv))
                    * (diff_v[i] - mv);

            /*
             * Estimated variance / accuracy.
             */
            let v_error =
                pv
                - (pv / (pv + rv)) * pv
                + DIV_EPSILON;

            /*
             * -------------------------------------------------------
             * Fuse the two directional estimates.
             *
             * This is exactly Getreuer's:
             *
             *     Green = Input + (V*h + H*v)/(H + V)
             *
             * The cross-weighting is intentional.
             * -------------------------------------------------------
             */
            green_channel[i] =
                buf.data[i]
                + (
                    v_error * h
                    + h_error * v
                ) / (
                    h_error + v_error
                );
        }
    }

    /*
     * ---------------------------------------------------------------
     * Primary difference signals.
     *
     * DiffGR = G - R at red locations
     * DiffGB = G - B at blue locations
     *
     * Getreuer aliases:
     *
     *     DiffGR = DiffH
     *     DiffGB = DiffV
     * ---------------------------------------------------------------
     */
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;

            if ((x + y) & 1) != green {
                if (y & 1) == red_y {
                    diff_h[i] =
                        green_channel[i]
                        - buf.data[i];
                } else {
                    diff_v[i] =
                        green_channel[i]
                        - buf.data[i];
                }
            }
        }
    }

    /*
     * ---------------------------------------------------------------
     * DiagonalAverage.
     *
     * Direct translation of Getreuer's function, including
     * whole-sample symmetric boundary handling.
     * ---------------------------------------------------------------
     */
    #[inline(always)]
    fn diagonal_average(
        image: &[f32],
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> f32 {
        if y == 0 {
            if x == 0 {
                image[1 + width]
            } else if x < width - 1 {
                (
                    image[x - 1 + width]
                    + image[x + 1 + width]
                ) / 2.0
            } else {
                image[x - 1 + width]
            }
        } else if y < height - 1 {
            if x == 0 {
                (
                    image[1 + (y - 1) * width]
                    + image[1 + (y + 1) * width]
                ) / 2.0
            } else if x < width - 1 {
                (
                    image[(x - 1) + (y - 1) * width]
                    + image[(x + 1) + (y - 1) * width]
                    + image[(x - 1) + (y + 1) * width]
                    + image[(x + 1) + (y + 1) * width]
                ) / 4.0
            } else {
                (
                    image[(x - 1) + (y - 1) * width]
                    + image[(x - 1) + (y + 1) * width]
                ) / 2.0
            }
        } else {
            if x == 0 {
                image[1 + (y - 1) * width]
            } else if x < width - 1 {
                (
                    image[(x - 1) + (y - 1) * width]
                    + image[(x + 1) + (y - 1) * width]
                ) / 2.0
            } else {
                image[(x - 1) + (y - 1) * width]
            }
        }
    }

    /*
     * ---------------------------------------------------------------
     * Interpolate the missing primary difference signals at the
     * opposite-color locations.
     *
     * At red locations:
     *     DiffGB is interpolated diagonally.
     *
     * At blue locations:
     *     DiffGR is interpolated diagonally.
     *
     * This is exactly the ordering in Getreuer's implementation.
     * ---------------------------------------------------------------
     */
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;

            if ((x + y) & 1) != green {
                if (y & 1) == red_y {
                    diff_v[i] =
                        diagonal_average(
                            &diff_v,
                            width,
                            height,
                            x,
                            y,
                        );
                } else {
                    diff_h[i] =
                        diagonal_average(
                            &diff_h,
                            width,
                            height,
                            x,
                            y,
                        );
                }
            }
        }
    }

    /*
     * ---------------------------------------------------------------
     * AxialAverage.
     *
     * Direct translation of Getreuer's function.
     * ---------------------------------------------------------------
     */
    #[inline(always)]
    fn axial_average(
        image: &[f32],
        width: usize,
        height: usize,
        x: usize,
        y: usize,
    ) -> f32 {
        if y == 0 {
            if x == 0 {
                (
                    image[1]
                    + image[width]
                ) / 2.0
            } else if x < width - 1 {
                (
                    image[x - 1]
                    + image[x + 1]
                    + 2.0 * image[x + width]
                ) / 4.0
            } else {
                (
                    image[x - 1]
                    + image[x + width]
                ) / 2.0
            }
        } else if y < height - 1 {
            let i = y * width + x;

            if x == 0 {
                (
                    2.0 * image[i + 1]
                    + image[i - width]
                    + image[i + width]
                ) / 4.0
            } else if x < width - 1 {
                (
                    image[i - 1]
                    + image[i + 1]
                    + image[i - width]
                    + image[i + width]
                ) / 4.0
            } else {
                (
                    2.0 * image[i - 1]
                    + image[i - width]
                    + image[i + width]
                ) / 4.0
            }
        } else {
            let i = y * width + x;

            if x == 0 {
                (
                    image[i + 1]
                    + image[i - width]
                ) / 2.0
            } else if x < width - 1 {
                (
                    image[i - 1]
                    + image[i + 1]
                    + 2.0 * image[i - width]
                ) / 4.0
            } else {
                (
                    image[i - 1]
                    + image[i - width]
                ) / 2.0
            }
        }
    }

    /*
     * ---------------------------------------------------------------
     * Interpolate both primary difference signals at green pixels.
     * ---------------------------------------------------------------
     */
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;

            if ((x + y) & 1) == green {
                diff_v[i] =
                    axial_average(
                        &diff_v,
                        width,
                        height,
                        x,
                        y,
                    );

                diff_h[i] =
                    axial_average(
                        &diff_h,
                        width,
                        height,
                        x,
                        y,
                    );
            }
        }
    }

    /*
     * ---------------------------------------------------------------
     * Recover R and B:
     *
     *     R = G - (G-R)
     *     B = G - (G-B)
     * ---------------------------------------------------------------
     */
    out.mutate_lines(&(|line: &mut [f32], row| {
        for x in 0..width {
            let i = row * width + x;
            let o = x * 4;

            line[o] =
                green_channel[i] - diff_h[i];

            line[o + 1] =
                green_channel[i];

            line[o + 2] =
                green_channel[i] - diff_v[i];

            // LMMSE is a 3-channel Bayer algorithm.
            // Keep imagepipe's 4-channel pipeline interface.
            line[o + 3] = 0.0;
        }
    }));

    out
}
