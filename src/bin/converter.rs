use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::time::Instant;
use image::ColorType;

extern crate imagepipe;
extern crate rawloader;
extern crate image;

fn usage() {
  println!("converter <file> [outfile]");
  std::process::exit(1);
}

fn error(err: &str) {
  println!("ERROR: {}", err);
  std::process::exit(2);
}

fn main() {
  let args: Vec<_> = env::args().collect();
  if args.len() < 2 {
    usage();
  }
  let file = &args[1];
  let filejpg = format!("{}.jpg",file);
  let outfile = if args.len() > 2 {
    &args[2]
  } else {
    &filejpg
  };
  println!("Loading file \"{}\" and saving it as \"{}\"", file, outfile);

  let from_time = Instant::now();
  let image = match rawloader::decode_file(file) {
    Ok(val) => val,
    Err(e) => {error(&e.to_string());unreachable!()},
  };
  let duration = from_time.elapsed();
  println!("Decoded in {} ms", duration.as_millis());

  println!("Found camera \"{}\" model \"{}\"", image.make, image.model);
  println!("Found clean named camera \"{}\" model \"{}\"", image.clean_make, image.clean_model);
  println!("Image size is {}x{}", image.width, image.height);
  println!("WB coeffs are {:?}", image.wb_coeffs);
  println!("black levels are {:?}", image.blacklevels);
  println!("white levels are {:?}", image.whitelevels);
  println!("xyz_to_cam is {:?}", image.xyz_to_cam);
  println!("CFA is {:?}", image.cfa);
  println!("crops are {:?}", image.crops);

  let decoded = match imagepipe::simple_decode_8bit(file, 0, 0) {
    Ok(img) => img,
    Err(e) => {error(&e);unreachable!()},
  };

  let uf = match File::create(outfile) {
    Ok(val) => val,
    Err(e) => {
      error(format!("Error: {}", e).as_ref());
      unreachable!()
    }
  };
  let mut f = BufWriter::new(uf);

  let mut jpg_encoder = image::jpeg::JpegEncoder::new_with_quality(&mut f, 80);
  jpg_encoder
    .encode(&decoded.data, decoded.width as u32, decoded.height as u32, ColorType::Rgb8)
    .expect("Encoding image in JPEG format failed.");
}
