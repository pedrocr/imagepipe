lazy_static! {
  pub static ref SRGB_D65_33: [[f32;3];3] = [
    [0.4124564, 0.3575761, 0.1804375],
    [0.2126729, 0.7151522, 0.0721750],
    [0.0193339, 0.1191920, 0.9503041],
  ];
  pub static ref SRGB_D65_XYZ_WHITE: (f32,f32,f32) = (0.95047, 1.000, 1.08883);
  pub static ref XYZ_D65_33: [[f32;3];3] = inverse(*SRGB_D65_33);
  pub static ref XYZ_D65_34: [[f32;3];4] = [
    XYZ_D65_33[0], XYZ_D65_33[1], XYZ_D65_33[2], [0.0, 0.0, 0.0]
  ];
  pub static ref SRGB_D65_43: [[f32;4];3] = [
    [SRGB_D65_33[0][0], SRGB_D65_33[0][1], SRGB_D65_33[0][2], 0.0],
    [SRGB_D65_33[1][0], SRGB_D65_33[1][1], SRGB_D65_33[1][2], 0.0],
    [SRGB_D65_33[2][0], SRGB_D65_33[2][1], SRGB_D65_33[2][2], 0.0],
  ];
}

// FIXME: when float math is allowed in const fn get rid of lazy_static!
fn inverse(inm: [[f32;3];3]) -> [[f32;3];3] {
  let invdet = 1.0 / (
    inm[0][0] * (inm[1][1] * inm[2][2] - inm[2][1] * inm[1][2]) -
    inm[0][1] * (inm[1][0] * inm[2][2] - inm[1][2] * inm[2][0]) +
    inm[0][2] * (inm[1][0] * inm[2][1] - inm[1][1] * inm[2][0])
  );

  let mut out = [[0.0; 3];3];
  out[0][0] =  (inm[1][1]*inm[2][2] - inm[2][1]*inm[1][2]) * invdet;
  out[0][1] = -(inm[0][1]*inm[2][2] - inm[0][2]*inm[2][1]) * invdet;
  out[0][2] =  (inm[0][1]*inm[1][2] - inm[0][2]*inm[1][1]) * invdet;
  out[1][0] = -(inm[1][0]*inm[2][2] - inm[1][2]*inm[2][0]) * invdet;
  out[1][1] =  (inm[0][0]*inm[2][2] - inm[0][2]*inm[2][0]) * invdet;
  out[1][2] = -(inm[0][0]*inm[1][2] - inm[1][0]*inm[0][2]) * invdet;
  out[2][0] =  (inm[1][0]*inm[2][1] - inm[2][0]*inm[1][1]) * invdet;
  out[2][1] = -(inm[0][0]*inm[2][1] - inm[2][0]*inm[0][1]) * invdet;
  out[2][2] =  (inm[0][0]*inm[1][1] - inm[1][0]*inm[0][1]) * invdet;

  out
}

#[inline(always)]
pub fn camera_to_lab(mul: [f32;4], cmatrix: [[f32;4];3], pixin: &[f32]) -> (f32, f32, f32) {
    // Multiply pixel by RGBE multipliers, clipping to 1.0
    let r = (pixin[0] * mul[0]).min(1.0);
    let g = (pixin[1] * mul[1]).min(1.0);
    let b = (pixin[2] * mul[2]).min(1.0);
    let e = (pixin[3] * mul[3]).min(1.0);

    // Calculate XYZ by applying the camera matrix
    let x = r * cmatrix[0][0] + g * cmatrix[0][1] + b * cmatrix[0][2] + e * cmatrix[0][3];
    let y = r * cmatrix[1][0] + g * cmatrix[1][1] + b * cmatrix[1][2] + e * cmatrix[1][3];
    let z = r * cmatrix[2][0] + g * cmatrix[2][1] + b * cmatrix[2][2] + e * cmatrix[2][3];

    xyz_to_lab(x,y,z)
}

#[inline(always)]
pub fn lab_to_rgb(rgbmatrix: [[f32;3];3], pixin: &[f32]) -> (f32, f32, f32) {
    let (x,y,z) = lab_to_xyz(pixin[0], pixin[1], pixin[2]);

    let r = x * rgbmatrix[0][0] + y * rgbmatrix[0][1] + z * rgbmatrix[0][2];
    let g = x * rgbmatrix[1][0] + y * rgbmatrix[1][1] + z * rgbmatrix[1][2];
    let b = x * rgbmatrix[2][0] + y * rgbmatrix[2][1] + z * rgbmatrix[2][2];
    (r, g, b)
}

#[inline(always)]
pub fn temp_tint_to_rgb(temp: f32, tint: f32) -> (f32, f32, f32) {
    let xyz = temp_to_xyz(temp);
    let (x, y , z) = (xyz[0], xyz[1]/tint, xyz[2]);

    let rgbmatrix = *XYZ_D65_33;
    let r = x * rgbmatrix[0][0] + y * rgbmatrix[0][1] + z * rgbmatrix[0][2];
    let g = x * rgbmatrix[1][0] + y * rgbmatrix[1][1] + z * rgbmatrix[1][2];
    let b = x * rgbmatrix[2][0] + y * rgbmatrix[2][1] + z * rgbmatrix[2][2];
    (r, g, b)
}

// Encapsulate a given transformation including a lookup table for speedup
struct TransformLookup {
  max: f32,
  table: Vec<f32>,
  transform: Box<dyn Fn(f32) -> f32+Sync>,
}

impl TransformLookup {
  fn new<F>(maxbits: usize, transform: F) -> Self
    where F : Fn(f32) -> f32+Sync+'static {
    let max = (1 << maxbits) - 1;
    let mut table: Vec<f32> = Vec::with_capacity(max+2);
    for i in 0..=(max+1) {
      let v = (i as f32) / (max as f32);
      table.push(transform(v));
    }
    Self {
      max: max as f32,
      table,
      transform: Box::new(transform),
    }
  }

  fn lookup(&self, val: f32) -> f32 {
    if val < 0.0 || val > 1.0 {
      (self.transform)(val)
    } else {
      let pos = val * self.max as f32;
      let key = pos as usize;
      let base = pos.trunc();
      let a = pos - base;
      let v1 = self.table[key];
      let v2 = self.table[key+1];
      v1 + a * (v2 - v1)
    }
  }
}

// FIXME: In the future when floats and loops are stable in const fn get rid of
//        lazy_static and have the table be generated at compile time instead.
lazy_static! {
  static ref XYZ_LAB_TRANSFORM: TransformLookup = TransformLookup::new(13, |v: f32| {
    let e = 216.0 / 24389.0;
    let k = 24389.0 / 27.0;
    if v > e {v.cbrt()} else {(k*v + 16.0) / 116.0}
  });

  static ref SRGB_GAMMA_REVERSE: TransformLookup = TransformLookup::new(13, |v: f32| {
    if v < 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
  });

  static ref SRGB_GAMMA_TRANSFORM: TransformLookup = TransformLookup::new(13, |v: f32| {
    if v < 0.0031308 {
      v * 12.92
    } else {
      1.055 * v.powf(1.0 / 2.4) - 0.055
    }
  });
}

/// Remove sRGB gamma from a value
#[inline(always)]
pub fn expand_srgb_gamma(v: f32) -> f32 {
  SRGB_GAMMA_REVERSE.lookup(v)
}

/// Apply sRGB gamma to a value
#[inline(always)]
pub fn apply_srgb_gamma(v: f32) -> f32 {
  SRGB_GAMMA_TRANSFORM.lookup(v)
}

#[inline(always)]
pub fn xyz_to_lab(x: f32, y: f32, z: f32) -> (f32,f32,f32) {
  let (xw, yw, zw) = *SRGB_D65_XYZ_WHITE;
  let (xr, yr, zr) = (x/xw, y/yw, z/zw);

  let fx = XYZ_LAB_TRANSFORM.lookup(xr);
  let fy = XYZ_LAB_TRANSFORM.lookup(yr);
  let fz = XYZ_LAB_TRANSFORM.lookup(zr);

  let l = 116.0 * fy - 16.0;
  let a = 500.0 * (fx - fy);
  let b = 200.0 * (fy - fz);

  (l/100.0,(a+127.0)/255.0,(b+127.0)/255.0)
}

#[inline(always)]
pub fn lab_to_xyz(l: f32, a: f32, b: f32) -> (f32,f32,f32) {
  let cl = l * 100.0;
  let ca = (a * 255.0) - 127.0;
  let cb = (b * 255.0) - 127.0;

  let fy = (cl + 16.0) / 116.0;
  let fx = ca / 500.0 + fy;
  let fz = fy - (cb / 200.0);

  let e = 216.0 / 24389.0;
  let k = 24389.0 / 27.0;
  let fx3 = fx * fx * fx;
  let xr = if fx3 > e {fx3} else {(116.0 * fx - 16.0)/k};
  let yr = if cl > k*e {fy*fy*fy} else {cl / k};
  let fz3 = fz * fz * fz;
  let zr = if fz3 > e {fz3} else {(116.0 * fz - 16.0)/k};

  let (xw, yw, zw) = *SRGB_D65_XYZ_WHITE;
  (xr*xw, yr*yw, zr*zw)
}

const CIE_OBSERVERS : [(u32, [f64;3]); 81] = [
  ( 380, [ 0.001368, 0.000039, 0.006450 ] ),
  ( 385, [ 0.002236, 0.000064, 0.010550 ] ),
  ( 390, [ 0.004243, 0.000120, 0.020050 ] ),
  ( 395, [ 0.007650, 0.000217, 0.036210 ] ),
  ( 400, [ 0.014310, 0.000396, 0.067850 ] ),
  ( 405, [ 0.023190, 0.000640, 0.110200 ] ),
  ( 410, [ 0.043510, 0.001210, 0.207400 ] ),
  ( 415, [ 0.077630, 0.002180, 0.371300 ] ),
  ( 420, [ 0.134380, 0.004000, 0.645600 ] ),
  ( 425, [ 0.214770, 0.007300, 1.039050 ] ),
  ( 430, [ 0.283900, 0.011600, 1.385600 ] ),
  ( 435, [ 0.328500, 0.016840, 1.622960 ] ),
  ( 440, [ 0.348280, 0.023000, 1.747060 ] ),
  ( 445, [ 0.348060, 0.029800, 1.782600 ] ),
  ( 450, [ 0.336200, 0.038000, 1.772110 ] ),
  ( 455, [ 0.318700, 0.048000, 1.744100 ] ),
  ( 460, [ 0.290800, 0.060000, 1.669200 ] ),
  ( 465, [ 0.251100, 0.073900, 1.528100 ] ),
  ( 470, [ 0.195360, 0.090980, 1.287640 ] ),
  ( 475, [ 0.142100, 0.112600, 1.041900 ] ),
  ( 480, [ 0.095640, 0.139020, 0.812950 ] ),
  ( 485, [ 0.057950, 0.169300, 0.616200 ] ),
  ( 490, [ 0.032010, 0.208020, 0.465180 ] ),
  ( 495, [ 0.014700, 0.258600, 0.353300 ] ),
  ( 500, [ 0.004900, 0.323000, 0.272000 ] ),
  ( 505, [ 0.002400, 0.407300, 0.212300 ] ),
  ( 510, [ 0.009300, 0.503000, 0.158200 ] ),
  ( 515, [ 0.029100, 0.608200, 0.111700 ] ),
  ( 520, [ 0.063270, 0.710000, 0.078250 ] ),
  ( 525, [ 0.109600, 0.793200, 0.057250 ] ),
  ( 530, [ 0.165500, 0.862000, 0.042160 ] ),
  ( 535, [ 0.225750, 0.914850, 0.029840 ] ),
  ( 540, [ 0.290400, 0.954000, 0.020300 ] ),
  ( 545, [ 0.359700, 0.980300, 0.013400 ] ),
  ( 550, [ 0.433450, 0.994950, 0.008750 ] ),
  ( 555, [ 0.512050, 1.000000, 0.005750 ] ),
  ( 560, [ 0.594500, 0.995000, 0.003900 ] ),
  ( 565, [ 0.678400, 0.978600, 0.002750 ] ),
  ( 570, [ 0.762100, 0.952000, 0.002100 ] ),
  ( 575, [ 0.842500, 0.915400, 0.001800 ] ),
  ( 580, [ 0.916300, 0.870000, 0.001650 ] ),
  ( 585, [ 0.978600, 0.816300, 0.001400 ] ),
  ( 590, [ 1.026300, 0.757000, 0.001100 ] ),
  ( 595, [ 1.056700, 0.694900, 0.001000 ] ),
  ( 600, [ 1.062200, 0.631000, 0.000800 ] ),
  ( 605, [ 1.045600, 0.566800, 0.000600 ] ),
  ( 610, [ 1.002600, 0.503000, 0.000340 ] ),
  ( 615, [ 0.938400, 0.441200, 0.000240 ] ),
  ( 620, [ 0.854450, 0.381000, 0.000190 ] ),
  ( 625, [ 0.751400, 0.321000, 0.000100 ] ),
  ( 630, [ 0.642400, 0.265000, 0.000050 ] ),
  ( 635, [ 0.541900, 0.217000, 0.000030 ] ),
  ( 640, [ 0.447900, 0.175000, 0.000020 ] ),
  ( 645, [ 0.360800, 0.138200, 0.000010 ] ),
  ( 650, [ 0.283500, 0.107000, 0.000000 ] ),
  ( 655, [ 0.218700, 0.081600, 0.000000 ] ),
  ( 660, [ 0.164900, 0.061000, 0.000000 ] ),
  ( 665, [ 0.121200, 0.044580, 0.000000 ] ),
  ( 670, [ 0.087400, 0.032000, 0.000000 ] ),
  ( 675, [ 0.063600, 0.023200, 0.000000 ] ),
  ( 680, [ 0.046770, 0.017000, 0.000000 ] ),
  ( 685, [ 0.032900, 0.011920, 0.000000 ] ),
  ( 690, [ 0.022700, 0.008210, 0.000000 ] ),
  ( 695, [ 0.015840, 0.005723, 0.000000 ] ),
  ( 700, [ 0.011359, 0.004102, 0.000000 ] ),
  ( 705, [ 0.008111, 0.002929, 0.000000 ] ),
  ( 710, [ 0.005790, 0.002091, 0.000000 ] ),
  ( 715, [ 0.004109, 0.001484, 0.000000 ] ),
  ( 720, [ 0.002899, 0.001047, 0.000000 ] ),
  ( 725, [ 0.002049, 0.000740, 0.000000 ] ),
  ( 730, [ 0.001440, 0.000520, 0.000000 ] ),
  ( 735, [ 0.001000, 0.000361, 0.000000 ] ),
  ( 740, [ 0.000690, 0.000249, 0.000000 ] ),
  ( 745, [ 0.000476, 0.000172, 0.000000 ] ),
  ( 750, [ 0.000332, 0.000120, 0.000000 ] ),
  ( 755, [ 0.000235, 0.000085, 0.000000 ] ),
  ( 760, [ 0.000166, 0.000060, 0.000000 ] ),
  ( 765, [ 0.000117, 0.000042, 0.000000 ] ),
  ( 770, [ 0.000083, 0.000030, 0.000000 ] ),
  ( 775, [ 0.000059, 0.000021, 0.000000 ] ),
  ( 780, [ 0.000042, 0.000015, 0.000000 ] ),
];

pub fn temp_to_xyz(temp: f32) -> [f32; 3] {
  const C1: f64 = 3.7417717905326694e-16;
  const C2: f64 = 0.014387773457709927;

  let mut xyz = [0.0f64; 3];
  for (wavelength, vals) in CIE_OBSERVERS.iter() {
    // Get the wavelength in meters
    let wavelength = (*wavelength as f64) / 1.0e9;
    let power = C1 / (wavelength.powi(5) * ((C2/((temp as f64)*wavelength)).exp()-1.0));
    xyz[0] += power * vals[0];
    xyz[1] += power * vals[1];
    xyz[2] += power * vals[2];
  }
  let max = xyz[0].max(xyz[1]).max(xyz[2]);

  [(xyz[0] / max) as f32, (xyz[1] / max) as f32, (xyz[2] / max) as f32]
}

pub fn xyz_to_temp(xyz: [f32; 3]) -> (f32, f32) {
  let (mut min, mut max) = (1000.0f32, 40000.0f32);
  let mut temp = 0.0;
  let mut new_xyz = [0.0; 3];
  while (max - min) > 1.0 {
    temp = (max + min) / 2.0;
    new_xyz = temp_to_xyz(temp);
    if (new_xyz[2] / new_xyz[0]) > (xyz[2] / xyz[0]) {
      max = temp;
    } else {
      min = temp;
    }
  }
  let tint = (new_xyz[1]/new_xyz[0]) / (xyz[1]/xyz[0]);
  (temp, tint)
}

#[inline(always)]
pub fn input8bit(v: u8) -> f32 {
  (v as f32) / 255.0
}

#[inline(always)]
pub fn input16bit(v: u16) -> f32 {
  (v as f32) / 65535.0
}

#[inline(always)]
pub fn output8bit(v: f32) -> u8 {
  (v * 256.0).max(0.0).min(255.0) as u8
}

#[inline(always)]
pub fn output16bit(v: f32) -> u16 {
  (v * 65535.0).round().max(0.0).min(65535.0) as u16
}

#[cfg(test)]
mod tests {
  use super::*;
  use image::{ImageBuffer, DynamicImage};

  #[test]
  fn roundtrip_8bit() {
    for i in 0..u8::MAX {
      assert_eq!(i, output8bit(input8bit(i)));
    }
  }

  #[test]
  fn roundtrip_16bit() {
    for i in 0..u16::MAX {
      assert_eq!(i, output16bit(input16bit(i)));
    }
  }

  // Make sure all 8 bit values roundtrip when read as 16 bits from the image crate
  #[test]
  fn roundtrip_8bit_values_image_crate() {
    let mut image_data: Vec<u8> = vec![0; 10*10*3];
    for v in 0..=u8::MAX {
      image_data.push(v);
    }
    let image = ImageBuffer::from_raw(10, 10, image_data.clone()).unwrap();
    let out = DynamicImage::ImageRgb8(image).into_rgb16().into_raw();

    for (vin, vout) in image_data.into_iter().zip(out.into_iter()) {
      let f = input16bit(vout);
      let out = output8bit(f);
      assert_eq!(out, vin);
    }
  }

  // Make sure all 16 bit values roundtrip when read as 16 bits from the image crate
  #[test]
  fn roundtrip_16bit_values_image_crate() {
    let mut image_data: Vec<u16> = vec![0; 148*148*3];
    for v in 0..=u16::MAX {
      image_data.push(v);
    }
    let image = ImageBuffer::from_raw(10, 10, image_data.clone()).unwrap();
    let out = DynamicImage::ImageRgb16(image).into_rgb16().into_raw();

    for (vin, vout) in image_data.into_iter().zip(out.into_iter()) {
      let f = input16bit(vout);
      let out = output16bit(f);
      assert_eq!(out, vin);
    }
  }

  fn roundtrip_gamma(v: f32) -> f32 {
    let inter = expand_srgb_gamma(v);
    apply_srgb_gamma(inter)
  }

  #[test]
  fn roundtrip_8bit_gamma() {
    for i in 0..u8::MAX {
      assert_eq!(i, output8bit(roundtrip_gamma(input8bit(i))));
    }
  }

  #[test]
  fn roundtrip_16bit_gamma() {
    for i in 0..u16::MAX {
      assert_eq!(i, output16bit(roundtrip_gamma(input16bit(i))));
    }
  }

  use num_traits::ops::saturating::Saturating;
  use std::fmt::Debug;
  use std::cmp::PartialOrd;
  #[inline(always)]
  fn assert_offby<T>(to: (T,T,T), from: (T,T,T), offdown: T, offup: T)
    where T: Saturating+Debug+PartialOrd+Copy {
    let condition =
      to.0 <= from.0.saturating_add(offup) && to.0 >= from.0.saturating_sub(offdown) &&
      to.1 <= from.1.saturating_add(offup) && to.1 >= from.1.saturating_sub(offdown) &&
      to.2 <= from.2.saturating_add(offup) && to.2 >= from.2.saturating_sub(offdown);
    if !condition {
      eprintln!("Got {:?} instead of {:?}", to, from);
    }
    assert!(condition)
  }

  #[test]
  fn roundtrip_8bit_lab_xyz() {
    for x in 0..u8::MAX {
      for y in 0..u8::MAX {
        for z in 0..u8::MAX {
          let xf = input8bit(x);
          let yf = input8bit(y);
          let zf = input8bit(z);

          let (l,a,b) = xyz_to_lab(xf,yf,zf);
          let (outxf,outyf,outzf) = lab_to_xyz(l,a,b);

          let outx = output8bit(outxf);
          let outy = output8bit(outyf);
          let outz = output8bit(outzf);

          assert_eq!((outx, outy, outz), (x, y, z));
        }
      }
    }
  }

  #[test]
  fn roundtrip_8bit_lab_rgb() {
    for r in 0..u8::MAX {
      for g in 0..u8::MAX {
        for b in 0..u8::MAX {
          let pixel = [input8bit(r), input8bit(g), input8bit(b), 0.0];
          let multipliers = [1.0,1.0,1.0,1.0];
          let cmatrix = *SRGB_D65_43;
          let rgbmatrix = *XYZ_D65_33;

          let (ll,la,lb) = camera_to_lab(multipliers, cmatrix, &pixel);
          let (outrf,outgf,outbf) = lab_to_rgb(rgbmatrix, &[ll,la,lb]);

          let outr = output8bit(outrf);
          let outg = output8bit(outgf);
          let outb = output8bit(outbf);

          assert_eq!((outr, outg, outb), (r, g, b));
        }
      }
    }
  }

  #[test]
  fn roundtrip_8bit_lab_rgb_gamma() {
    for r in 0..u8::MAX {
      for g in 0..u8::MAX {
        for b in 0..u8::MAX {
          let pixel = [
            expand_srgb_gamma(input8bit(r)),
            expand_srgb_gamma(input8bit(g)),
            expand_srgb_gamma(input8bit(b)),
            0.0
          ];
          let multipliers = [1.0,1.0,1.0,1.0];
          let cmatrix = *SRGB_D65_43;
          let rgbmatrix = *XYZ_D65_33;

          let (ll,la,lb) = camera_to_lab(multipliers, cmatrix, &pixel);
          let (outrf,outgf,outbf) = lab_to_rgb(rgbmatrix, &[ll,la,lb]);

          let outrf = apply_srgb_gamma(outrf);
          let outgf = apply_srgb_gamma(outgf);
          let outbf = apply_srgb_gamma(outbf);

          let outr = output8bit(outrf);
          let outg = output8bit(outgf);
          let outb = output8bit(outbf);

          assert_eq!((outr, outg, outb), (r, g, b));
        }
      }
    }
  }

  #[test]
  fn roundtrip_16bit_lab_xyz() {
    // step_by different primes to try and get coverage without being exaustive
    for x in (0..u16::MAX).step_by(89) {
      for y in (0..u16::MAX).step_by(97){
        for z in (0..u16::MAX).step_by(101) {
          let xf = input16bit(x);
          let yf = input16bit(y);
          let zf = input16bit(z);

          let (l,a,b) = xyz_to_lab(xf,yf,zf);
          let (outxf,outyf,outzf) = lab_to_xyz(l,a,b);

          // test output 16 bit
          let outx = output16bit(outxf);
          let outy = output16bit(outyf);
          let outz = output16bit(outzf);

          assert_eq!((outx, outy, outz), (x, y, z));

          // test output 8 bit
          let x = x >> 8;
          let y = y >> 8;
          let z = z >> 8;

          let outx = output8bit(outxf) as u16;
          let outy = output8bit(outyf) as u16;
          let outz = output8bit(outzf) as u16;

          assert_eq!((outx, outy, outz), (x, y, z));
        }
      }
    }
  }

  #[test]
  fn roundtrip_16bit_lab_rgb() {
    for r in (0..u16::MAX).step_by(89) {
      for g in (0..u16::MAX).step_by(97){
        for b in (0..u16::MAX).step_by(101) {
          let pixel = [input16bit(r), input16bit(g), input16bit(b), 0.0];
          let multipliers = [1.0,1.0,1.0,1.0];
          let cmatrix = *SRGB_D65_43;
          let rgbmatrix = *XYZ_D65_33;

          let (ll,la,lb) = camera_to_lab(multipliers, cmatrix, &pixel);
          let (outrf,outgf,outbf) = lab_to_rgb(rgbmatrix, &[ll,la,lb]);

          // test output 16 bit
          let outr = output16bit(outrf);
          let outg = output16bit(outgf);
          let outb = output16bit(outbf);

          assert_eq!((outr, outg, outb), (r, g, b));

          // test output 8 bit
          let r = r >> 8;
          let g = g >> 8;
          let b = b >> 8;

          let outr = output8bit(outrf) as u16;
          let outg = output8bit(outgf) as u16;
          let outb = output8bit(outbf) as u16;

          assert_eq!((outr, outg, outb), (r, g, b));
        }
      }
    }
  }

  #[test]
  fn roundtrip_16bit_lab_rgb_gamma() {
    for r in (0..u16::MAX).step_by(89) {
      for g in (0..u16::MAX).step_by(97){
        for b in (0..u16::MAX).step_by(101) {
          let pixel = [
            expand_srgb_gamma(input16bit(r)),
            expand_srgb_gamma(input16bit(g)),
            expand_srgb_gamma(input16bit(b)),
            0.0
          ];
          let multipliers = [1.0,1.0,1.0,1.0];
          let cmatrix = *SRGB_D65_43;
          let rgbmatrix = *XYZ_D65_33;

          let (ll,la,lb) = camera_to_lab(multipliers, cmatrix, &pixel);
          let ll = roundtrip_gamma(ll);
          let (outrf,outgf,outbf) = lab_to_rgb(rgbmatrix, &[ll,la,lb]);

          let outrf = apply_srgb_gamma(outrf);
          let outgf = apply_srgb_gamma(outgf);
          let outbf = apply_srgb_gamma(outbf);

          // test output 16 bit
          let outr = output16bit(outrf) as u16;
          let outg = output16bit(outgf) as u16;
          let outb = output16bit(outbf) as u16;

          // FIXME: 16bit full color conversion has off-by-one roundtrip only
          assert_offby((outr, outg, outb), (r, g, b), 1, 1);

          // test output 8 bit
          let r = r >> 8;
          let g = g >> 8;
          let b = b >> 8;

          let outr = output8bit(outrf) as u16;
          let outg = output8bit(outgf) as u16;
          let outb = output8bit(outbf) as u16;

          assert_eq!((outr, outg, outb), (r, g, b));
        }
      }
    }
  }
}
