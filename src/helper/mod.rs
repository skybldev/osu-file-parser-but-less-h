pub mod trait_ext;

use std::num::ParseIntError;

use thiserror::Error;

use crate::osu_file::{Version, VersionedToString};

pub const OLD_VERSION_TIME_OFFSET: u32 = 24;

pub fn add_old_version_time_offset(t: u32, version: Version) -> u32 {
    if (3..=4).contains(&version) {
        t + OLD_VERSION_TIME_OFFSET
    } else {
        t
    }
}

pub fn pipe_vec_to_string<T>(vec: &[T], version: Version) -> String
where
    T: VersionedToString,
{
    vec.iter()
        .map(|s| s.to_string(version).unwrap())
        .collect::<Vec<_>>()
        .join("|")
}

pub fn check_flag_at_bit_u8(value: u8, nth_bit: u8) -> bool {
    value >> nth_bit & 1 == 1
}

pub fn parse_zero_one_bool(value: &str) -> Result<bool, ParseZeroOneBoolError> {
    let value = value.parse()?;

    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ParseZeroOneBoolError::InvalidValue),
    }
}

#[derive(Debug, Error)]
pub enum ParseZeroOneBoolError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error("Error parsing value as `true` or `false`, expected value of 0 or 1")]
    InvalidValue,
}
