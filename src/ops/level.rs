use opbasics::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpLevel {
  pub blacklevels: [f32;4],
  pub whitelevels: [f32;4],
  pub wb_coeffs: [f32;4],
}

fn from_int4(arr: [u16;4]) -> [f32;4] {
  [arr[0] as f32, arr[1] as f32, arr[2] as f32, arr[3] as f32]
}

fn normalize_wbs(vals: [f32;4]) -> [f32;4] {
  // Set green multiplier as 1.0
  let unity: f32 = vals[1];

  macro_rules! norm {
    ($val:expr) => {
      if !$val.is_normal() {
        1.0
      } else {
        $val / unity
      }
    };
  }

  [norm!(vals[0]), norm!(vals[1]), norm!(vals[2]), norm!(vals[3])]
}

impl OpLevel {
  pub fn new(img: &RawImage) -> OpLevel {
    let coeffs = if !img.wb_coeffs[0].is_normal() ||
                    !img.wb_coeffs[1].is_normal() ||
                    !img.wb_coeffs[2].is_normal() {
      img.neutralwb()
    } else {
      normalize_wbs(img.wb_coeffs)
    };

    OpLevel{
      blacklevels: from_int4(img.blacklevels),
      whitelevels: from_int4(img.whitelevels),
      wb_coeffs: coeffs,
    }
  }
}

impl<'a> ImageOp<'a> for OpLevel {
  fn name(&self) -> &str {"level"}
  fn run(&self, _pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    // Calculate the levels
    let mins = self.blacklevels;
    let ranges = self.whitelevels.iter().enumerate().map(|(i, &x)| {
      x - mins[i]
    }).collect::<Vec<f32>>();

    let mul = if buf.monochrome {
      [1.0, 1.0, 1.0, 1.0]
    } else {
      normalize_wbs(self.wb_coeffs)
    };

    Arc::new(buf.mutate_lines_copying(&(|line: &mut [f32], _| {
      for pix in line.chunks_mut(4) {
        pix[0] = (((pix[0] - mins[0]) / ranges[0]) * mul[0]).min(1.0);
        pix[1] = (((pix[1] - mins[1]) / ranges[1]) * mul[1]).min(1.0);
        pix[2] = (((pix[2] - mins[2]) / ranges[2]) * mul[2]).min(1.0);
        pix[3] = (((pix[3] - mins[3]) / ranges[3]) * mul[3]).min(1.0);
      }
    })))
  }
}
