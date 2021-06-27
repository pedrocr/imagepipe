extern crate rayon;
use self::rayon::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct OpBuffer {
  pub width: usize,
  pub height: usize,
  pub colors: usize,
  pub monochrome: bool,
  pub data: Vec<f32>,
}

impl OpBuffer {
  pub fn default() -> OpBuffer {
    OpBuffer {
      width: 0,
      height: 0,
      colors: 0,
      monochrome: false,
      data: Vec::new(),
    }
  }

  pub fn new(width: usize, height: usize, colors: usize, monochrome: bool) -> OpBuffer {
    OpBuffer {
      width: width,
      height: height,
      colors: colors,
      monochrome,
      data: vec![0.0; width*height*(colors as usize)],
    }
  }

  pub fn mutate_lines<F>(&mut self, closure: &F)
    where F : Fn(&mut [f32], usize)+Sync {

    self.data.par_chunks_mut(self.width*self.colors).enumerate().for_each(|(row, line)| {
      closure(line, row);
    });
  }

  pub fn mutate_lines_copying<F>(&self, closure: &F) -> OpBuffer
    where F : Fn(&mut [f32], usize)+Sync {

    let mut buf = self.clone();
    buf.data.par_chunks_mut(self.width*self.colors).enumerate().for_each(|(row, line)| {
      closure(line, row);
    });
    buf
  }

  pub fn process_into_new<F>(&self, colors: usize, closure: &F) -> OpBuffer
    where F : Fn(&mut [f32], &[f32])+Sync {

    let mut out = OpBuffer::new(self.width, self.height, colors, self.monochrome);
    out.data.par_chunks_mut(out.width*out.colors).enumerate().for_each(|(row, line)| {
      closure(line, &self.data[self.width*self.colors*row..]);
    });
    out
  }

  pub fn transform(&self,
    topleft: (isize, isize),
    topright: (isize, isize),
    bottomleft: (isize, isize),
    width: usize,
    height: usize) -> OpBuffer {

    let newdata = crate::scaling::transform_buffer(&self.data, self.width, self.height,
      topleft, topright, bottomleft, width, height, self.colors, None);

    Self {
      width,
      height,
      colors: self.colors,
      monochrome: self.monochrome,
      data: newdata,
    }
  }

  /// Helper function to allow human readable creation of `OpBuffer` instances
  pub fn from_rgb_str_vec(data: Vec<&str>) -> OpBuffer {
    let width = data.first().expect("Invalid data for rgb helper function").len();
    let height = data.len();
    let colors = 3;

    let mut pixel_data: Vec<f32> = Vec::with_capacity(width * height * colors);
    for row in data {
      for col in row.chars() {
        let (r, g, b) = match col {
            'R' => (1.0, 0.0, 0.0),
            'G' => (0.0, 1.0, 0.0),
            'B' => (0.0, 0.0, 1.0),
            'O' => (1.0, 1.0, 1.0),
            ' ' => (0.0, 0.0, 0.0),
            c @ _ => panic!(
              "Invalid color '{}' sent to rgb expected any of 'RGBO '", c),
        };

        pixel_data.push(r);
        pixel_data.push(g);
        pixel_data.push(b);
      }
    }

    OpBuffer {
      width: width,
      height: height,
      colors: colors,
      monochrome: false,
      data: pixel_data,
    }
  }
}
