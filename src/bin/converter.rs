use std::env;
use std::fs::File;
use std::error::Error;
use std::io::prelude::*;
use std::io::BufWriter;
extern crate time;
extern crate imagepipe;
extern crate rawloader;

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
  let fileppm = format!("{}.ppm",file);
  let outfile = if args.len() > 2 {
    &args[2]
  } else {
    &fileppm
  };
  println!("Loading file \"{}\" and saving it as \"{}\"", file, outfile);

  let from_time = time::precise_time_ns();
  let image = match rawloader::decode_file(file) {
    Ok(val) => val,
    Err(e) => {error(&e.to_string());unreachable!()},
  };
  let to_time = time::precise_time_ns();
  println!("Decoded in {} ms", (to_time - from_time)/1000000);

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
    Err(e) => {error(e.description());unreachable!()},
  };
  let mut f = BufWriter::new(uf);
  let preamble = format!("P6 {} {} {}\n", decoded.width, decoded.height, 255).into_bytes();
  if let Err(err) = f.write_all(&preamble) {
    error(err.description());
  }
  if let Err(err) = f.write_all(&decoded.data) {
    error(err.description());
  }
}
