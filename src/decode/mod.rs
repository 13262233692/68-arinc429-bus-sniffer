pub mod bnr;
pub mod bcd;
pub mod discrete;

pub use bnr::*;
pub use bcd::*;
pub use discrete::*;

use crate::core::types::EngineeringValue;

pub trait Decoder {
    fn decode(&self, data: u32, ssm_sign: i32) -> Option<EngineeringValue>;
}
