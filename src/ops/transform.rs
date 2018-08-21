use opbasics::*;

use std::mem;
use std::usize;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Rotation {
  Normal,
  Rotate90,
  Rotate180,
  Rotate270,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpTransform {
  pub rotation: Rotation,
  pub fliph: bool,
  pub flipv: bool,
}

impl OpTransform {
  pub fn new(img: &RawImage) -> OpTransform {
    let (rotation, fliph, flipv) = match img.orientation {
      Orientation::Normal
      | Orientation::Unknown      => (Rotation::Normal, false, false),
      Orientation::VerticalFlip   => (Rotation::Normal, false, true),
      Orientation::HorizontalFlip => (Rotation::Normal, true, false),
      Orientation::Rotate180      => (Rotation::Rotate180, false, false),
      Orientation::Transpose      => (Rotation::Rotate90, false, true),
      Orientation::Rotate90       => (Rotation::Rotate90, false, false),
      Orientation::Rotate270      => (Rotation::Rotate270, false, false),
      Orientation::Transverse     => (Rotation::Rotate270, true, false),
    };

    OpTransform{
      rotation,
      fliph,
      flipv,
    }
  }
}

impl<'a> ImageOp<'a> for OpTransform {
  fn name(&self) -> &str {"transform"}
  fn run(&self, _pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    // Grab back a base orientation
    let (f1, f2, f3) = match self.rotation {
      Rotation::Normal    => Orientation::Normal,
      Rotation::Rotate90  => Orientation::Rotate90,
      Rotation::Rotate180 => Orientation::Rotate180,
      Rotation::Rotate270 => Orientation::Rotate270,
    }.to_flips();

    // Adjust it with the vertical and horizontal flips if that applies
    let orientation = Orientation::from_flips((f1, f2 ^ self.fliph, f3 ^ self.flipv));

    if orientation == Orientation::Normal || orientation == Orientation::Unknown {
      buf
    } else {
      Arc::new(rotate_buffer(&buf, &orientation))
    }
  }
}

fn rotate_buffer(buf: &OpBuffer, orientation: &Orientation) -> OpBuffer {
  assert_eq!(buf.colors, 3); // When we're rotating we're always at 3 cpp

  // Don't rotate things we don't know how to rotate or don't need to
  if *orientation == Orientation::Normal || *orientation == Orientation::Unknown {
    return buf.clone();
  }

  // Since we are using isize when calculating values for the rotation its
  // indices must be addressable by an isize as well
  if buf.data.len() >= usize::MAX / 2 {
    panic!("Buffer is too wide or high to rotate");
  }

  // We extract buffers parameters early since all math is done with isize.
  // This avoids verbose casts later on
  let mut width = buf.width as isize;
  let mut height = buf.height as isize;

  let (transpose, flip_x, flip_y) = orientation.to_flips();

  let mut base_offset: isize = 0;
  let mut x_step: isize = 3;
  let mut y_step: isize = width * 3;

  if flip_x {
    x_step = -x_step;
    base_offset += (width - 1) * 3;
  }

  if flip_y {
    y_step = -y_step;
    base_offset += width * (height - 1) * 3;
  }

  let mut out = if transpose {
    mem::swap(&mut width, &mut height);
    mem::swap(&mut x_step, &mut y_step);
    OpBuffer::new(buf.height, buf.width, 3 as usize)
  } else {
    OpBuffer::new(buf.width, buf.height, 3 as usize)
  };

  out.mutate_lines(&(|line: &mut [f32], row| {
    // Calculate the current line's offset in original buffer. When transposing
    // this is the current column's offset in the original buffer
    let line_offset = base_offset + y_step * row as isize;
    for col in 0..width {
      // The current pixel's offset in original buffer
      let offset = line_offset + x_step * col;
      for c in 0..3 {
        line[(col * 3 + c) as usize] = buf.data[(offset + c) as usize];
      }
    }
  }));

  out
}

#[cfg(test)]
mod tests {
  use rawloader::Orientation;
  use buffer::OpBuffer;
  use super::rotate_buffer;

  // Store a colorful capital F as a constant, since it is used in all tests
  lazy_static! {
      static ref F: OpBuffer = {
        OpBuffer::from_rgb_str_vec(vec![
          "        ",
          " RRRRRR ",
          " GG     ",
          " BBBB   ",
          " GG     ",
          " GG     ",
          "        ",
        ])
      };
  }

  #[test]
  fn rotate_unknown() {
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::Unknown), F.clone());
  }

  #[test]
  fn rotate_normal() {
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::Normal), F.clone());
  }

  #[test]
  fn rotate_flip_x() {
    let output = OpBuffer::from_rgb_str_vec(vec![
      "        ",
      " RRRRRR ",
      "     GG ",
      "   BBBB ",
      "     GG ",
      "     GG ",
      "        ",
    ]);

    assert_eq!(rotate_buffer(&F.clone(), &Orientation::HorizontalFlip), output);
  }

  #[test]
  fn rotate_flip_y() {
    let output = OpBuffer::from_rgb_str_vec(vec![
      "        ",
      " GG     ",
      " GG     ",
      " BBBB   ",
      " GG     ",
      " RRRRRR ",
      "        ",
    ]);
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::VerticalFlip), output);
  }

  #[test]
  fn rotate_rotate90_cw() {
    let output = OpBuffer::from_rgb_str_vec(vec![
      "       ",
      " GGBGR ",
      " GGBGR ",
      "   B R ",
      "   B R ",
      "     R ",
      "     R ",
      "       ",
    ]);
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::Rotate90), output);
  }

  #[test]
  fn rotate_rotate270_cw() {
    let output = OpBuffer::from_rgb_str_vec(vec![
      "       ",
      " R     ",
      " R     ",
      " R B   ",
      " R B   ",
      " RGBGG ",
      " RGBGG ",
      "       ",
    ]);
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::Rotate270), output);
  }

  #[test]
  fn rotate_rotate180() {
    let output = OpBuffer::from_rgb_str_vec(vec![
      "        ",
      "     GG ",
      "     GG ",
      "   BBBB ",
      "     GG ",
      " RRRRRR ",
      "        ",
    ]);
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::Rotate180), output);
  }

  #[test]
  fn rotate_transpose() {
    let output = OpBuffer::from_rgb_str_vec(vec![
      "       ",
      " RGBGG ",
      " RGBGG ",
      " R B   ",
      " R B   ",
      " R     ",
      " R     ",
      "       ",
    ]);
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::Transpose), output);
  }

  #[test]
  fn rotate_transverse() {
    let output = OpBuffer::from_rgb_str_vec(vec![
      "       ",
      "     R ",
      "     R ",
      "   B R ",
      "   B R ",
      " GGBGR ",
      " GGBGR ",
      "       ",
    ]);
    assert_eq!(rotate_buffer(&F.clone(), &Orientation::Transverse), output);
  }
}
