extern crate blake3;

extern crate bincode;
extern crate serde;
use self::serde::Serialize;

use std;
use std::io::Write;
use std::fmt;
use std::fmt::Debug;

type HashType = self::blake3::Hasher;
const HASHSIZE: usize = 32;
pub type BufHash = [u8;HASHSIZE];

#[derive(Clone)]
pub struct BufHasher {
  hash: HashType,
}
impl BufHasher {
  pub fn new() -> BufHasher {
    BufHasher {
      hash: HashType::new(),
    }
  }
  pub fn result(&self) -> BufHash {
	*self.hash.finalize().as_bytes()
  }
}
impl Debug for BufHasher {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "BufHasher {{ {:?} }}", self.result())
  }
}

impl Write for BufHasher {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.hash.update(buf);
    Ok(buf.len())
  }
  fn flush(&mut self) -> std::io::Result<()> {Ok(())}
}

impl BufHasher {
  pub fn from_serialize<T>(&mut self, obj: &T) where T: Serialize {
    self::bincode::serialize_into(self, obj).unwrap();
  }
}
