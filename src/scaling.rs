use crate::buffer::*;
use crate::pipeline::{SRGBImage, SRGBImage16};
use rawloader::CFA;
use num_traits::cast::AsPrimitive;
use rayon::prelude::*;

pub fn calculate_scaling(width: usize, height: usize, maxwidth: usize, maxheight: usize) -> (f32, usize, usize) {
  if maxwidth == 0 && maxheight == 0 {
    (1.0, width, height)
  } else {
    // Do the calculations manually to avoid off-by-one errors from floating point rounding
    let xscale = if maxwidth == 0 {1.0} else {width as f32 / maxwidth as f32};
    let yscale = if maxheight == 0 {1.0} else {height as f32 / maxheight as f32};
    if yscale <= 1.0 && xscale <= 1.0 {
      (1.0, width, height)
    } else if yscale > xscale {
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

#[inline(always)]
fn scale_down_buffer<T>(
  src: &[T],
  width: usize,
  height: usize,
  nwidth: usize,
  nheight: usize,
  components: usize,
  cfa: Option<&CFA>,
  ) -> Vec<T>
  where f32: AsPrimitive<T>, T: AsPrimitive<f32>, T: Sync+Send {
  let mut out = vec![(0 as f32).as_(); nwidth*nheight*components];
  let rowskip = (width as f32) / (nwidth as f32);
  let colskip = (height as f32) / (nheight as f32);
  // Using rayon to make this multithreaded is 10-15% faster on an i5-6200U which
  // is useful but not a great speedup for 2 cores 4 threads. It may even make
  // sense to give this up to not thrash caches.
  out.par_chunks_exact_mut(nwidth*components).enumerate().for_each(|(row, line)| {
    for col in 0..nwidth {
      let mut sums: [f32; 4] = [0.0;4];
      let mut counts: [f32; 4] = [0.0;4];
      let (fromrow, torow, topfactor, bottomfactor) = calc_skips(row, height, rowskip);
      for y in fromrow..torow {
        let (fromcol, tocol, leftfactor, rightfactor) = calc_skips(col, width, colskip);
        for x in fromcol..tocol {
          let factor = {
            (if y == fromrow {topfactor} else if y == torow {bottomfactor} else {1.0}) *
            (if x == fromcol {leftfactor} else if x == tocol {rightfactor} else {1.0})
          };

            if let Some(cfa) = cfa {
              let c = cfa.color_at(y, x);
              sums[c] += src[y*width+x].as_() * factor;
              counts[c] += factor;
           } else {
              for c in 0..components {
                sums[c] += src[(y*width+x)*components+c].as_() * factor;
                counts[c] += factor;
              }
           }
        }
      }

      for c in 0..components {
        if counts[c] > 0.0 {
          line[col*components+c] = (sums[c] / counts[c]).as_();
        }
      }
    }
  });
  out
}

pub fn scaled_demosaic(cfa: CFA, buf: &OpBuffer, nwidth: usize, nheight: usize) -> OpBuffer {
  assert_eq!(buf.colors, 1); // When we're in demosaic we start with a 1 color buffer

  log::debug!("Doing a scaled demosaic from {}x{} to {}x{}", buf.width, buf.height, nwidth, nheight);
  let data = scale_down_buffer(&buf.data, buf.width, buf.height, nwidth, nheight, 4, Some(&cfa));

  OpBuffer {
    width: nwidth,
    height: nheight,
    data,
    monochrome: buf.monochrome,
    colors: 4,
  }
}

pub fn scale_down_opbuf(buf: &OpBuffer, nwidth: usize, nheight: usize) -> OpBuffer {
  assert_eq!(buf.colors, 4); // When we're scaling down we're always at 4 cpp

  log::debug!("Scaling OpBuffer from {}x{} to {}x{}", buf.width, buf.height, nwidth, nheight);
  let data = scale_down_buffer(&buf.data, buf.width, buf.height, nwidth, nheight, 4, None);

  OpBuffer {
    width: nwidth,
    height: nheight,
    data,
    monochrome: buf.monochrome,
    colors: 4,
  }
}

pub fn scale_down_srgb(buf: &SRGBImage, nwidth: usize, nheight: usize) -> SRGBImage {
  log::debug!("Scaling SRGBImage from {}x{} to {}x{}", buf.width, buf.height, nwidth, nheight);
  let data = scale_down_buffer(&buf.data, buf.width, buf.height, nwidth, nheight, 3, None);

  SRGBImage {
    width: nwidth,
    height: nheight,
    data,
  }
}

pub fn scale_down_srgb16(buf: &SRGBImage16, nwidth: usize, nheight: usize) -> SRGBImage16 {
  log::debug!("Scaling SRGBImage from {}x{} to {}x{}", buf.width, buf.height, nwidth, nheight);
  let data = scale_down_buffer(&buf.data, buf.width, buf.height, nwidth, nheight, 3, None);

  SRGBImage16 {
    width: nwidth,
    height: nheight,
    data,
  }
}
