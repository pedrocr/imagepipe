use crate::opbasics::*;

static EPSILON: f32 = 0.0000001;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpRotateCrop {
  pub crop_top: f32,
  pub crop_right: f32,
  pub crop_bottom: f32,
  pub crop_left: f32,
}

impl OpRotateCrop {
  pub fn new(_img: &ImageSource) -> Self {
    Self {
      crop_top: 0.0,
      crop_right: 0.0,
      crop_bottom: 0.0,
      crop_left: 0.0,
    }
  }
}

impl<'a> ImageOp<'a> for OpRotateCrop {
  fn name(&self) -> &str {"rotatecrop"}
  fn run(&self, _pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    if self.noop() { return buf; }

    let x = ((buf.width as f32) * self.crop_left).floor();
    if x < 0.0 || x > buf.width as f32 {
      log::error!("Trying to crop left outside image");
      return buf;
    }
    let y = ((buf.height as f32) * self.crop_top).floor();
    if y < 0.0 || y > buf.height as f32 {
      log::error!("Trying to crop top outside image");
      return buf;
    }
    let width = (buf.width as f32) - (buf.width as f32) * (self.crop_left + self.crop_right);
    if width < 1.0 || width > buf.width as f32 {
      log::error!("Trying to crop width beyond limits");
      return buf;
    }
    let height = (buf.height as f32) - (buf.height as f32) * (self.crop_top + self.crop_bottom);
    if height < 1.0 || height > buf.height as f32 {
      log::error!("Trying to crop height beyond limits");
      return buf;
    }
    let (x, y, width, height) = (x as usize, y as usize, width as usize, height as usize);
    let mut newbuffer = OpBuffer::new(width, height, buf.colors, buf.monochrome);
    newbuffer.mutate_lines(&(|line: &mut [f32], row| {
      for (o, i) in line.chunks_exact_mut(buf.colors)
        .zip(buf.data[(buf.width*(row+y)+x)*buf.colors..].chunks_exact(buf.colors)) {
        o.copy_from_slice(i);
      }
    }));
    Arc::new(newbuffer)
  }

  fn transform_forward(&self, width: usize, height: usize) -> (usize, usize) {
    if self.noop() { return (width, height); }
    let nwidth = (width as f32) * (1.0 - self.crop_left - self.crop_right);
    if nwidth < 1.0 || nwidth > width as f32 {
      log::error!("Trying to crop width beyond limits");
      return (width, height);
    }
    let nheight = (height as f32) * (1.0 - self.crop_top - self.crop_bottom);
    if nheight < 1.0 || nheight > height as f32 {
      log::error!("Trying to crop height beyond limits");
      return (width, height);
    }
    (nwidth as usize, nheight as usize)
  }

  fn transform_reverse(&self, width: usize, height: usize) -> (usize, usize) {
    if self.noop() { return (width, height); }
    let nwidth = (width as f32) / (1.0 - self.crop_left - self.crop_right);
    if nwidth < 1.0 {
      log::error!("Trying to crop width beyond limits");
      return (width, height);
    }
    let nheight = (height as f32) / (1.0 - self.crop_top - self.crop_bottom);
    if nheight < 1.0 {
      log::error!("Trying to crop height beyond limits");
      return (width, height);
    }
    (nwidth as usize, nheight as usize)
  }
}

impl OpRotateCrop {
  fn noop(&self) -> bool {
    self.crop_top.abs() < EPSILON &&
    self.crop_right.abs() < EPSILON &&
    self.crop_bottom.abs() < EPSILON &&
    self.crop_left.abs() < EPSILON
  }
}

#[cfg(test)]
mod tests {
  use crate::buffer::OpBuffer;
  use super::*;

  fn setup() -> (Arc<OpBuffer>, OpRotateCrop, PipelineGlobals) {
    let mut buffer = OpBuffer::new(100, 100, 3, false);
    for (i, o) in buffer.data.chunks_exact_mut(1).enumerate() {
      o[0] = i as f32;
    }
    let op = OpRotateCrop {
      crop_top: 0.0,
      crop_right: 0.0,
      crop_bottom: 0.0,
      crop_left: 0.0,
    };
    (Arc::new(buffer), op, PipelineGlobals::mock(100, 100))
  }

  #[test]
  fn crop_top() {
    let (buffer, mut op, globals) = setup();
    op.crop_top = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 90);
    assert_eq!(newbuf.width, 100);
    assert_eq!(&newbuf.data[0], &buffer.data[100*10*3]);
  }

  #[test]
  fn crop_bottom() {
    let (buffer, mut op, globals) = setup();
    op.crop_bottom = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 90);
    assert_eq!(newbuf.width, 100);
    assert_eq!(&newbuf.data[0], &buffer.data[0]);
  }

  #[test]
  fn crop_vertical() {
    let (buffer, mut op, globals) = setup();
    op.crop_top = 0.1;
    op.crop_bottom = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 80);
    assert_eq!(newbuf.width, 100);
    assert_eq!(&newbuf.data[0], &buffer.data[100*10*3]);
  }

  #[test]
  fn crop_left() {
    let (buffer, mut op, globals) = setup();
    op.crop_left = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 100);
    assert_eq!(newbuf.width, 90);
    assert_eq!(&newbuf.data[0], &buffer.data[10*3]);
  }

  #[test]
  fn crop_right() {
    let (buffer, mut op, globals) = setup();
    op.crop_right = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 100);
    assert_eq!(newbuf.width, 90);
    assert_eq!(&newbuf.data[0], &buffer.data[0]);
  }

  #[test]
  fn crop_horizontal() {
    let (buffer, mut op, globals) = setup();
    op.crop_left = 0.1;
    op.crop_right = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 100);
    assert_eq!(newbuf.width, 80);
    assert_eq!(&newbuf.data[0], &buffer.data[10*3]);
  }

  #[test]
  fn crop_horizontal_and_vertical() {
    let (buffer, mut op, globals) = setup();
    op.crop_left = 0.1;
    op.crop_right = 0.1;
    op.crop_top = 0.1;
    op.crop_bottom = 0.1;
    let newbuf = op.run(&globals, buffer.clone());
    assert_eq!(newbuf.height, 80);
    assert_eq!(newbuf.width, 80);
    assert_eq!(&newbuf.data[0], &buffer.data[100*10*3+10*3]);
  }
}
