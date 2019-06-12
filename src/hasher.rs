extern crate blake2;
use self::blake2::digest::{Input, VariableOutput};

extern crate bincode;
extern crate serde;
use self::serde::Serialize;

use std;
use std::io::Write;
use std::fmt;
use std::fmt::Debug;

type HashType = self::blake2::VarBlake2b;
const HASHSIZE: usize = 32;
pub type BufHash = [u8;HASHSIZE];

#[derive(Clone)]
pub struct BufHasher {
  hash: HashType,
}
impl BufHasher {
  pub fn new() -> BufHasher {
    BufHasher {
      hash: HashType::new(HASHSIZE).unwrap(),
    }
  }
  pub fn result(&self) -> BufHash {
    let mut result = BufHash::default();
    let hash = self.hash.clone();
    hash.variable_result(|res| {
      result.copy_from_slice(res);
    });
    result
  }
}
impl Debug for BufHasher {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "BufHasher {{ {:?} }}", self.result())
  }
}

impl Write for BufHasher {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.hash.input(buf);
    Ok(buf.len())
  }
  fn flush(&mut self) -> std::io::Result<()> {Ok(())}
}

impl BufHasher {
  pub fn from_serialize<T>(&mut self, obj: &T) where T: Serialize {
    self::bincode::serialize_into(self, obj).unwrap();
  }
}
