use crate::buffer::*;
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

pub fn scale_down_opbuf(buf: &OpBuffer, nwidth: usize, nheight: usize) -> OpBuffer {
  assert_eq!(buf.colors, 4); // When we're scaling down we're always at 4 cpp

  let mut out = OpBuffer::new(nwidth, nheight, 4, buf.monochrome);
  let rowskip = (buf.width as f32) / (nwidth as f32);
  let colskip = (buf.height as f32) / (nheight as f32);

  // Go around the image averaging blocks of pixels
  out.mutate_lines(&(|line: &mut [f32], row| {
    for col in 0..nwidth {
      let mut sums: [f32; 4] = [0.0;4];
      let mut counts: [f32; 4] = [0.0;4];
      let (fromrow, torow, topfactor, bottomfactor) = calc_skips(row, buf.height, rowskip);
      for y in fromrow..torow {
        let (fromcol, tocol, leftfactor, rightfactor) = calc_skips(col, buf.width, colskip);
        for x in fromcol..tocol {
          let factor = {
            (if y == fromrow {topfactor} else if y == torow {bottomfactor} else {1.0}) *
            (if x == fromcol {leftfactor} else if x == tocol {rightfactor} else {1.0})
          };

          for c in 0..4 {
            sums[c] += buf.data[(y*buf.width+x)*4 + c] * factor;
            counts[c] += factor;
          }
        }
      }

      for c in 0..4 {
        if counts[c] > 0.0 {
          line[col*4+c] = sums[c] / counts[c];
        }
      }
    }
  }));

  out
}

pub fn scaled_demosaic(cfa: CFA, buf: &OpBuffer, nwidth: usize, nheight: usize) -> OpBuffer {
  let mut out = OpBuffer::new(nwidth, nheight, 4, buf.monochrome);

  let rowskip = (buf.width as f32) / (nwidth as f32);
  let colskip = (buf.height as f32) / (nheight as f32);

  // Go around the image averaging blocks of pixels
  out.mutate_lines(&(|line: &mut [f32], row| {
    for col in 0..nwidth {
      let mut sums: [f32; 4] = [0.0;4];
      let mut counts: [f32; 4] = [0.0;4];
      let (fromrow, torow, topfactor, bottomfactor) = calc_skips(row, buf.height, rowskip);
      for y in fromrow..torow {
        let (fromcol, tocol, leftfactor, rightfactor) = calc_skips(col, buf.width, colskip);
        for x in fromcol..tocol {
          let factor = {
            (if y == fromrow {topfactor} else if y == torow {bottomfactor} else {1.0}) *
            (if x == fromcol {leftfactor} else if x == tocol {rightfactor} else {1.0})
          };

          let c = cfa.color_at(y, x);
          sums[c] += (buf.data[y*buf.width+x] as f32) * factor;
          counts[c] += factor;
        }
      }

      for c in 0..4 {
        if counts[c] > 0.0 {
          line[col*4+c] = sums[c] / counts[c];
        }
      }
    }
  }));

  out
}