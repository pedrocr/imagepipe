use crate::opbasics::*;
mod dumb;
mod mhc;
mod lmmse;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpDemosaic {
  pub cfa: String,
}

impl OpDemosaic {
  pub fn new(img: &ImageSource) -> OpDemosaic {
    match img {
      ImageSource::Raw(img) => {
        OpDemosaic{
          cfa: img.cropped_cfa().to_string(),
        }
      },
      ImageSource::Other(_) => {
        OpDemosaic{
          cfa: "".to_string(),
        }
      }
    }
  }
}

impl<'a> ImageOp<'a> for OpDemosaic {
  fn name(&self) -> &str {"demosaic"}
  fn run(&self, pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    let nwidth = pipeline.settings.demosaic_width;
    let nheight = pipeline.settings.demosaic_height;
    let scale = crate::scaling::calculate_scale(buf.width, buf.height, nwidth, nheight);

    let cfa = CFA::new(&self.cfa);
    let minscale = match cfa.width {
      2  => 2.0,  // RGGB/RGBE bayer
      6  => 3.0,  // x-trans is 6 wide but has all colors in every 3x3 block
      8  => 2.0,  // Canon pro 70 has a 8x2 patern that has all four colors every 2x2 block
      12 => 12.0, // some crazy sensor I haven't actually encountered, use full block
      _  => 2.0,  // default
    };

    if scale <= 1.0 && buf.colors == 4 {
      // We want full size and the image is already 4 color, pass it through
      buf
    } else if buf.colors == 4 {
      // Scale down a 4 colour image
      Arc::new(crate::scaling::scale_down_opbuf(&buf, nwidth, nheight))
    } else if scale >= minscale {
      // We're scaling down enough that each pixel has all four colors under it so do the
      // demosaic and scale down in one go
      Arc::new(crate::scaling::scaled_demosaic(cfa, &buf, nwidth, nheight))
    } else {
      // We're in a close to full scale output that needs full demosaic and possibly
      // minimal scale down
      let fullsize = if cfa.width == 2 && cfa.height ==2 && cfa.num_colors == 3 {
        // Normal Bayer sensor, use the better quality LMMSE
        lmmse::lmmse(&cfa, &buf)
      } else {
        // Everyone else gets the dumb generic implementation
        dumb::dumb(&cfa, &buf)
      };
      if scale > 1.0 {
        Arc::new(crate::scaling::scale_down_opbuf(&fullsize, nwidth, nheight))
      } else {
        Arc::new(fullsize)
      }
    }
  }

  // We don't transform_reverse as image sizing is relative to the scaling done
  // at the demosaic step, so whatever scale down is needed can be achieved here
}
