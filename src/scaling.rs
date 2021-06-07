use crate::buffer::*;
use crate::pipeline::SRGBImage;
use rawloader::CFA;

pub fn calculate_scaling(width: usize, height: usize, maxwidth: usize, maxheight: usize) -> (f32, usize, usize) {
  if maxwidth == 0 && maxheight == 0 {
    (1.0, width, height)
  } else {
    // Do the calculations manually to avoid off-by-one errors from floating point rounding
    let xscale = if maxwidth == 0 {1.0} else {width as f32 / maxwidth as f32};
    let yscale = if maxheight == 0 {1.0} else {height as f32 / maxheight as f32};
    if yscale > xscale {
      (yscale, ((width as f32)/yscale) as usize, maxheight)
    } else {
      (xscale, maxwidth, ((height as f32)/xscale) as usize)
    }
  }
}

fn calc_skips(idx: usize, idxmax: usize, skip: f32) -> (usize, usize, f32, f32) {
  let from = (idx as f32)*skip;
  let fromback = from.floor();
  let fromfactor = 1.0 - (from-fromback).fract();

  let to = ((idx+1) as f32)*skip;
  let toforward = (idxmax as f32).min(to.ceil());
  let tofactor = (toforward-to).fract();

  (fromback as usize, toforward as usize, fromfactor, tofactor)
}

macro_rules! scale_down_buffer {
  (
    $buf:expr,
    $nwidth:expr,
    $nheight:expr,
    $sum:ident,
    $arg:expr,
    $output_type:ty,
    $components:literal
  ) => {
    {
      let mut out = vec![0 as $output_type; $nwidth*$nheight*$components];
      let rowskip = ($buf.width as f32) / ($nwidth as f32);
      let colskip = ($buf.height as f32) / ($nheight as f32);
      for (row,line) in out.chunks_exact_mut($nwidth*$components).enumerate() {
        for col in 0..$nwidth {
          let mut sums: [f32; 4] = [0.0;4];
          let mut counts: [f32; 4] = [0.0;4];
          let (fromrow, torow, topfactor, bottomfactor) = calc_skips(row, $buf.height, rowskip);
          for y in fromrow..torow {
            let (fromcol, tocol, leftfactor, rightfactor) = calc_skips(col, $buf.width, colskip);
            for x in fromcol..tocol {
              let factor = {
                (if y == fromrow {topfactor} else if y == torow {bottomfactor} else {1.0}) *
                (if x == fromcol {leftfactor} else if x == tocol {rightfactor} else {1.0})
              };

              $sum(&$buf.data, &mut sums, &mut counts, x, y, $buf.width, factor, $arg);
            }
          }

          for c in 0..$components {
            if counts[c] > 0.0 {
              line[col*$components+c] = (sums[c] / counts[c]) as $output_type;
            }
          }
        }
      }
      out
    }
  }
}

#[inline(always)]
fn sum_cfa_1_f32(src: &[f32], sums: &mut [f32; 4], counts: &mut [f32; 4], x: usize, y: usize, width: usize, factor: f32, cfa: &CFA) {
  let c = cfa.color_at(y, x);
  sums[c] += (src[y*width+x] as f32) * factor;
  counts[c] += factor;
}

pub fn scaled_demosaic(cfa: CFA, buf: &OpBuffer, nwidth: usize, nheight: usize) -> OpBuffer {
  log::debug!("Doing a scaled demosaic from {}x{} to {}x{}", buf.width, buf.height, nwidth, nheight);
  assert_eq!(buf.colors, 1); // When we're in demosaic we start with a 1 color buffer

  let data = scale_down_buffer!(buf, nwidth, nheight, sum_cfa_1_f32, &cfa, f32, 4);

  OpBuffer {
    width: nwidth,
    height: nheight,
    data,
    monochrome: buf.monochrome,
    colors: 4,
  }
}

#[inline(always)]
fn sum_4_f32(src: &[f32], sums: &mut [f32; 4], counts: &mut [f32; 4], x: usize, y: usize, width: usize, factor: f32, _extra: ()) {
  for c in 0..4 {
    sums[c] += src[(y*width+x)*4+c] as f32 * factor;
    counts[c] += factor;
  }
}

pub fn scale_down_opbuf(buf: &OpBuffer, nwidth: usize, nheight: usize) -> OpBuffer {
  log::debug!("Scaling OpBuffer from {}x{} to {}x{}", buf.width, buf.height, nwidth, nheight);
  assert_eq!(buf.colors, 4); // When we're scaling down we're always at 4 cpp

  let data = scale_down_buffer!(buf, nwidth, nheight, sum_4_f32, (), f32, 4);

  OpBuffer {
    width: nwidth,
    height: nheight,
    data,
    monochrome: buf.monochrome,
    colors: 4,
  }
}

#[inline(always)]
fn sum_3_u8(src: &[u8], sums: &mut [f32; 4], counts: &mut [f32; 4], x: usize, y: usize, width: usize, factor: f32, _extra: ()) {
  for c in 0..3 {
    sums[c] += src[(y*width+x)*3+c] as f32 * factor;
    counts[c] += factor;
  }
}

pub fn scale_down_srgb(buf: &SRGBImage, nwidth: usize, nheight: usize) -> SRGBImage {
  log::debug!("Scaling SRGBImage from {}x{} to {}x{}", buf.width, buf.height, nwidth, nheight);
  let data = scale_down_buffer!(buf, nwidth, nheight, sum_3_u8, (), u8, 3);

  SRGBImage {
    width: nwidth,
    height: nheight,
    data,
  }
}
