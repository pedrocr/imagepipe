use crate::opbasics::*;
pub fn dumb(cfa: &CFA, buf: &OpBuffer) -> OpBuffer {
  let mut out = OpBuffer::new(buf.width, buf.height, 4, buf.monochrome);

  let offsets3x3: [(isize,isize);9] = [
    (-1,-1), (-1, 0), (-1, 1),
    ( 0,-1), ( 0, 0), ( 0, 1),
    ( 1,-1), ( 1, 0), ( 1, 1),
  ];

  // Initialize a lookup table for the colors of each pixel in a 3x3 grid
  let mut lookups = [[[0;9];48];48];
  for (row, line) in lookups.iter_mut().enumerate() {
    for (col, colors) in line.iter_mut().enumerate() {
      let pixcolor = cfa.color_at(row, col);

      for (i, o) in offsets3x3.iter().enumerate() {
        let (dy, dx) = *o;
        let row = (48+dy) as usize + row;
        let col = (48+dx) as usize + col;
        let ocolor = cfa.color_at(row, col);
        colors[i] = if ocolor != pixcolor || (dx == 0 && dy == 0) { ocolor } else { 4 };
      }
    }
  }

  // Now calculate RGBE for each pixel based on the lookup table
  out.mutate_lines(&(|line: &mut [f32], row| {
    for (col, pix) in line.chunks_exact_mut(4).enumerate() {
      let ref colors = lookups[row%48][col%48];
      let mut sums = [0f32;5];
      let mut counts = [0f32;5];

      for (i, o) in offsets3x3.iter().enumerate() {
        let (dy, dx) = *o;
        let row = row as isize + dy;
        let col = col as isize + dx;
        if row >= 0 && row < (buf.height as isize) &&
           col >= 0 && col < (buf.width as isize) {
          sums[colors[i]] += buf.data[(row as usize)*buf.width+(col as usize)];
          counts[colors[i]] += 1.0;
        }
      }

      for c in 0..4 {
        if counts[c] > 0.0 {
          pix[c] = sums[c] / counts[c];
        }
      }
    }
  }));

  out
}
