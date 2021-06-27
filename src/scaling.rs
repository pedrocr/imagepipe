use crate::buffer::*;
use crate::pipeline::{SRGBImage, SRGBImage16};
use rawloader::CFA;
use num_traits::cast::AsPrimitive;
use rayon::prelude::*;
use std::cmp;

fn calculate_scaling_total(width: usize, height: usize, maxwidth: usize, maxheight: usize) -> (f32, usize, usize) {
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

pub fn scaling_size(width: usize, height: usize, maxwidth: usize, maxheight: usize) -> (usize, usize) {
  let (_, width, height) = calculate_scaling_total(width, height, maxwidth, maxheight);
  (width, height)
}

pub fn calculate_scale(width: usize, height: usize, maxwidth: usize, maxheight: usize) -> f32 {
  calculate_scaling_total(width, height, maxwidth, maxheight).0
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

  transform_buffer(src, width, height, (0, 0), (width as isize - 1, 0), (0, height as isize - 1),
    nwidth, nheight, components, cfa)
}

#[inline(always)]
pub fn transform_buffer<T>(
  src: &[T],
  width: usize,
  height: usize,
  topleft: (isize, isize),
  topright: (isize, isize),
  bottomleft: (isize, isize),
  nwidth: usize,
  nheight: usize,
  components: usize,
  cfa: Option<&CFA>,
  ) -> Vec<T>
  where f32: AsPrimitive<T>, T: AsPrimitive<f32>, T: Sync+Send {
  let mut out = vec![(0 as f32).as_(); nwidth*nheight*components];

  // This scales by using a rectangular window of the source image for each
  // destination pixel. The destination pixel is filled with a weighted average
  // of the source window, using the square of the distance as the weight.
  let skip_x_x = (topright.0 as f32- topleft.0 as f32) / ((nwidth-1) as f32);
  let skip_x_y = (topright.1 as f32 - topleft.1 as f32) / ((nwidth-1) as f32);
  let skip_y_x = (bottomleft.0 as f32 - topleft.0 as f32) / ((nheight-1) as f32);
  let skip_y_y = (bottomleft.1 as f32 - topleft.1 as f32) / ((nheight-1) as f32);
  // Using rayon to make this multithreaded is 10-15% faster on an i5-6200U which
  // is useful but not a great speedup for 2 cores 4 threads. It may even make
  // sense to give this up to not thrash caches.
  out.par_chunks_exact_mut(nwidth*components).enumerate().for_each(|(row, line)| {
    let from_x = topleft.0 as f32 + skip_y_x * row as f32;
    let to_x = topleft.0 as f32 + skip_y_x * (row+1) as f32;
    let from_y = topleft.1 as f32 + skip_y_y * row as f32;
    let to_y = topleft.1 as f32 + skip_y_y * (row+1) as f32;
    let center_x = (topleft.0 as f32) + (skip_y_x * row as f32) + (skip_y_x / 2.0) - 0.5;
    let center_y = (topleft.1 as f32) + (skip_y_y * row as f32) + (skip_y_y / 2.0) - 0.5;
    for col in 0..nwidth {
      let from_x = cmp::min(width-1, (from_x + (skip_x_x * col as f32)).floor() as usize);
      let to_x = cmp::min(width-1, (to_x + (skip_x_x * (col+1) as f32)).floor() as usize);
      let from_y = cmp::min(height-1, (from_y + (skip_x_y * col as f32)).floor() as usize);
      let to_y = cmp::min(height-1, (to_y + (skip_x_y * (col+1) as f32)).floor() as usize);
      let center_x = center_x + (skip_x_x * col as f32) + (skip_x_x / 2.0);
      let center_y = center_y + (skip_x_y * col as f32) + (skip_x_y / 2.0);

      let mut sums = [0.0 as f32; 4];
      let mut counts = [0.0 as f32; 4];
      for y in from_y..=to_y {
        for x in from_x..=to_x {
          // FIXME: Hopefully this is a reasonable low-pass filter that works for
          //        most cases but something more sophisticated may be useful.
          //        More specifically probably one of two things:
          //        - A gaussian filter with parameters calculated based on how
          //          much scale down we are doing so as to exactly remove the
          //          high frequencies we can no longer represent
          //        - A good windowed sinc function like Lanczos that should
          //          preserve more detail but will always have some artifacts
          //          in some cases
          let delta_x = (x as f32 - center_x) / skip_x_x;
          let delta_y = (y as f32 - center_y) / skip_y_y;
          let factor = 1.0 - (delta_x*delta_x) - (delta_y*delta_y);
          let factor = if factor < 0.0 {0.0} else {factor};

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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scaling_noop() {
    let width = 150;
    let height = 150;
    let mut data = vec![0 as u16; width*height*3];
    for (i, o) in data.chunks_exact_mut(1).enumerate() {
      o[0] = i as u16;
    }
    let orig = SRGBImage16 {
      width,
      height,
      data,
    };
    let new = scale_down_srgb16(&orig, width, height);
    assert_eq!(orig, new);
  }
}
