use crate::opbasics::*;

// Unused MHC vibe coded test implementation
#[allow(dead_code)]
pub fn mhc(cfa: &CFA, buf: &OpBuffer) -> OpBuffer {
    assert!(
        cfa.width == 2 && cfa.height == 2,
        "MHC requires a 2x2 Bayer CFA"
    );

    assert!(
        buf.colors == 1,
        "MHC expects a single-channel CFA buffer"
    );

    assert!(
        buf.width >= 2 && buf.height >= 2,
        "MHC requires an image at least 2x2 pixels"
    );

    struct Kernel {
        // Fixed coefficients for neighboring samples.
        terms: &'static [(isize, isize, f32)],

        // Coefficients applied to the center sample, conditional
        // on the corresponding neighboring sample being present.
        center_terms: &'static [(isize, isize, f32)],

        // Samples whose presence determines the normalization.
        denom: &'static [(isize, isize)],

        // Multiplier for the denominator.
        denom_scale: f32,
    }

    #[inline(always)]
    fn present(
        row: usize,
        col: usize,
        dy: isize,
        dx: isize,
        width: usize,
        height: usize,
    ) -> bool {
        let y = row as isize + dy;
        let x = col as isize + dx;

        y >= 0
            && y < height as isize
            && x >= 0
            && x < width as isize
    }

    #[inline(always)]
    fn sample(
        buf: &OpBuffer,
        row: usize,
        col: usize,
        dy: isize,
        dx: isize,
    ) -> f32 {
        buf.data[
            (row as isize + dy) as usize * buf.width
                + (col as isize + dx) as usize
        ]
    }

    #[inline(always)]
    fn apply_kernel(
        buf: &OpBuffer,
        row: usize,
        col: usize,
        kernel: &Kernel,
    ) -> f32 {
        let mut sum = 0.0f32;

        // Fixed neighboring coefficients.
        for &(dy, dx, coeff) in kernel.terms {
            if present(row, col, dy, dx, buf.width, buf.height) {
                sum += coeff * sample(buf, row, col, dy, dx);
            }
        }

        // The center coefficient depends on which surrounding
        // samples are actually available.
        let mut center_coeff = 0.0f32;

        for &(dy, dx, coeff) in kernel.center_terms {
            if present(row, col, dy, dx, buf.width, buf.height) {
                center_coeff += coeff;
            }
        }

        sum += center_coeff * sample(buf, row, col, 0, 0);

        // Boundary-dependent normalization.
        let mut count = 0.0f32;

        for &(dy, dx) in kernel.denom {
            if present(row, col, dy, dx, buf.width, buf.height) {
                count += 1.0;
            }
        }

        sum / (kernel.denom_scale * count)
    }

    /*
     * MHC: green at red/blue.
     *
     * Interior kernel, scaled by 8:
     *
     *   0  0 -1  0  0
     *   0  0  2  0  0
     *  -1  2  4  2 -1
     *   0  0  2  0  0
     *   0  0 -1  0  0
     *
     * The center coefficient is 1 for each available
     * distance-2 axial sample, giving 4 in the interior.
     */
    static G_AT_RB_TERMS: [(isize, isize, f32); 8] = [
        (-1,  0,  2.0),
        ( 1,  0,  2.0),
        ( 0, -1,  2.0),
        ( 0,  1,  2.0),

        (-2,  0, -1.0),
        ( 2,  0, -1.0),
        ( 0, -2, -1.0),
        ( 0,  2, -1.0),
    ];

    static G_AT_RB_CENTER: [(isize, isize, f32); 4] = [
        (-2,  0, 1.0),
        ( 2,  0, 1.0),
        ( 0, -2, 1.0),
        ( 0,  2, 1.0),
    ];

    static G_AT_RB_DENOM: [(isize, isize); 4] = [
        (-1,  0),
        ( 1,  0),
        ( 0, -1),
        ( 0,  1),
    ];

    static G_AT_RB: Kernel = Kernel {
        terms: &G_AT_RB_TERMS,
        center_terms: &G_AT_RB_CENTER,
        denom: &G_AT_RB_DENOM,
        denom_scale: 2.0,
    };


    /*
     * MHC: red at blue, or blue at red.
     *
     * Interior kernel, scaled by 8:
     *
     *   0  0 -3  0  0
     *   0  4  0  4  0
     *  -3  0  12 0 -3
     *   0  4  0  4  0
     *   0  0 -3  0  0
     *
     * Equivalently this is:
     *
     * 4 * diagonal opposite-color samples
     * + 3 * (center - distance-2 axial samples)
     *
     * divided by 4 * number of diagonal samples.
     */
    static RB_AT_OPPOSITE_TERMS: [(isize, isize, f32); 8] = [
        (-1, -1,  4.0),
        (-1,  1,  4.0),
        ( 1, -1,  4.0),
        ( 1,  1,  4.0),

        (-2,  0, -3.0),
        ( 2,  0, -3.0),
        ( 0, -2, -3.0),
        ( 0,  2, -3.0),
    ];

    static RB_AT_OPPOSITE_CENTER: [(isize, isize, f32); 4] = [
        (-2,  0, 3.0),
        ( 2,  0, 3.0),
        ( 0, -2, 3.0),
        ( 0,  2, 3.0),
    ];

    static RB_AT_OPPOSITE_DENOM: [(isize, isize); 4] = [
        (-1, -1),
        (-1,  1),
        ( 1, -1),
        ( 1,  1),
    ];

    static RB_AT_OPPOSITE: Kernel = Kernel {
        terms: &RB_AT_OPPOSITE_TERMS,
        center_terms: &RB_AT_OPPOSITE_CENTER,
        denom: &RB_AT_OPPOSITE_DENOM,
        denom_scale: 4.0,
    };


    /*
     * MHC: red at green where red is horizontal.
     *
     * Interior kernel, scaled by 8:
     *
     *   0  0  1  0  0
     *   0 -1  0 -1  0
     *  -1  4  5  4 -1
     *   0 -1  0 -1  0
     *   0  0  1  0  0
     */
    static R_AT_G_H_TERMS: [(isize, isize, f32); 10] = [
        // Horizontal red samples.
        ( 0, -1,  8.0),
        ( 0,  1,  8.0),

        // Four diagonal green samples.
        (-1, -1, -2.0),
        (-1,  1, -2.0),
        ( 1, -1, -2.0),
        ( 1,  1, -2.0),

        // Horizontal distance-2 green samples.
        ( 0, -2, -2.0),
        ( 0,  2, -2.0),

        // Vertical distance-2 green samples.
        (-2,  0,  1.0),
        ( 2,  0,  1.0),
    ];

    static R_AT_G_H_CENTER: [(isize, isize, f32); 8] = [
        (-1, -1,  2.0),
        (-1,  1,  2.0),
        ( 1, -1,  2.0),
        ( 1,  1,  2.0),

        ( 0, -2,  2.0),
        ( 0,  2,  2.0),

        (-2,  0, -1.0),
        ( 2,  0, -1.0),
    ];

    static R_AT_G_H_DENOM: [(isize, isize); 2] = [
        (0, -1),
        (0,  1),
    ];

    static R_AT_G_H: Kernel = Kernel {
        terms: &R_AT_G_H_TERMS,
        center_terms: &R_AT_G_H_CENTER,
        denom: &R_AT_G_H_DENOM,
        denom_scale: 8.0,
    };


    /*
     * MHC: red at green where red is vertical.
     *
     * This is the transpose of R_AT_G_H.
     */
    static R_AT_G_V_TERMS: [(isize, isize, f32); 10] = [
        // Vertical red samples.
        (-1,  0,  8.0),
        ( 1,  0,  8.0),

        // Four diagonal green samples.
        (-1, -1, -2.0),
        (-1,  1, -2.0),
        ( 1, -1, -2.0),
        ( 1,  1, -2.0),

        // Vertical distance-2 green samples.
        (-2,  0, -2.0),
        ( 2,  0, -2.0),

        // Horizontal distance-2 green samples.
        ( 0, -2,  1.0),
        ( 0,  2,  1.0),
    ];

    static R_AT_G_V_CENTER: [(isize, isize, f32); 8] = [
        (-1, -1,  2.0),
        (-1,  1,  2.0),
        ( 1, -1,  2.0),
        ( 1,  1,  2.0),

        (-2,  0,  2.0),
        ( 2,  0,  2.0),

        ( 0, -2, -1.0),
        ( 0,  2, -1.0),
    ];

    static R_AT_G_V_DENOM: [(isize, isize); 2] = [
        (-1, 0),
        ( 1, 0),
    ];

    static R_AT_G_V: Kernel = Kernel {
        terms: &R_AT_G_V_TERMS,
        center_terms: &R_AT_G_V_CENTER,
        denom: &R_AT_G_V_DENOM,
        denom_scale: 8.0,
    };


    /*
     * Blue at green is identical to red at green, with the
     * red/blue roles exchanged.
     */
    static B_AT_G_H: Kernel = Kernel {
        terms: &R_AT_G_H_TERMS,
        center_terms: &R_AT_G_H_CENTER,
        denom: &R_AT_G_H_DENOM,
        denom_scale: 8.0,
    };

    static B_AT_G_V: Kernel = Kernel {
        terms: &R_AT_G_V_TERMS,
        center_terms: &R_AT_G_V_CENTER,
        denom: &R_AT_G_V_DENOM,
        denom_scale: 8.0,
    };


    let mut out = OpBuffer::new(
        buf.width,
        buf.height,
        4,
        buf.monochrome,
    );

    out.mutate_lines(&(|line: &mut [f32], row| {
        for col in 0..buf.width {
            let p = col * 4;
            let input = buf.data[row * buf.width + col];

            match cfa.color_at(row, col) {
                // --------------------------------------------------
                // RED SAMPLE
                // --------------------------------------------------
                0 => {
                    let g = apply_kernel(
                        buf,
                        row,
                        col,
                        &G_AT_RB,
                    );

                    let b = apply_kernel(
                        buf,
                        row,
                        col,
                        &RB_AT_OPPOSITE,
                    );

                    line[p]     = input;
                    line[p + 1] = g;
                    line[p + 2] = b;
                    line[p + 3] = 0.0;
                }

                // --------------------------------------------------
                // BLUE SAMPLE
                // --------------------------------------------------
                2 => {
                    let g = apply_kernel(
                        buf,
                        row,
                        col,
                        &G_AT_RB,
                    );

                    let r = apply_kernel(
                        buf,
                        row,
                        col,
                        &RB_AT_OPPOSITE,
                    );

                    line[p]     = r;
                    line[p + 1] = g;
                    line[p + 2] = input;
                    line[p + 3] = 0.0;
                }

                // --------------------------------------------------
                // GREEN SAMPLE
                // --------------------------------------------------
                1 => {
                    /*
                     * Determine whether the horizontal neighbors
                     * are red or blue. This avoids making any
                     * assumptions about the Bayer phase.
                     *
                     * For a normal Bayer CFA, at least one horizontal
                     * neighbor exists because we require width >= 2.
                     */
                    let horizontal_color = if col > 0 {
                        cfa.color_at(row, col - 1)
                    } else {
                        cfa.color_at(row, col + 1)
                    };

                    if horizontal_color == 0 {
                        // R - G - R horizontally.
                        // B is vertical.

                        let r = apply_kernel(
                            buf,
                            row,
                            col,
                            &R_AT_G_H,
                        );

                        let b = apply_kernel(
                            buf,
                            row,
                            col,
                            &B_AT_G_V,
                        );

                        line[p]     = r;
                        line[p + 1] = input;
                        line[p + 2] = b;
                        line[p + 3] = 0.0;
                    } else {
                        // B - G - B horizontally.
                        // R is vertical.

                        let r = apply_kernel(
                            buf,
                            row,
                            col,
                            &R_AT_G_V,
                        );

                        let b = apply_kernel(
                            buf,
                            row,
                            col,
                            &B_AT_G_H,
                        );

                        line[p]     = r;
                        line[p + 1] = input;
                        line[p + 2] = b;
                        line[p + 3] = 0.0;
                    }
                }

                _ => unreachable!("MHC only supports R/G/B Bayer CFA"),
            }
        }
    }));

    out
}
