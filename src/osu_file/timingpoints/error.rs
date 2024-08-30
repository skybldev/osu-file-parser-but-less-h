use std::num::ParseIntError;

use strum_macros::{EnumString, IntoStaticStr};
use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct ParseError(#[from] ParseTimingPointError);

/// Error used when there was a problem parsing the [`TimingPoint`][super::TimingPoint].
#[derive(Debug, Error, EnumString, IntoStaticStr)]
#[non_exhaustive]
pub enum ParseTimingPointError {
    /// Invalid `time` value.
    #[error("Invalid `time` value")]
    InvalidTime,
    /// Invalid `beat_length` field.
    #[error("Missing `beat_length` field")]
    InvalidBeatLength,
    /// Invalid `meter` value.
    #[error("Invalid `meter` value")]
    InvalidMeter,
    /// Invalid `sample_set` value.
    #[error("Invalid `sample_set` value")]
    InvalidSampleSet,
    /// Invalid `sample_index` value.
    #[error("Invalid `sample_index` value")]
    InvalidSampleIndex,
    /// Invalid `volume` value.
    #[error("Invalid `volume` value")]
    InvalidVolume,
    /// Invalid `effects` value.
    #[error("Invalid `effects` value")]
    InvalidEffects,
    /// Invalid `uninherited` value.
    #[error("Invalid `uninherited` value")]
    InvalidUninherited,
    /// Invalid field count.
    #[error("The number of fields in the timing point is invalid.")]
    InvalidFieldCount
}

/// There was some problem parsing the [`SampleSet`][super::SampleSet].
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseSampleSetError {
    /// The value failed to parse from a `str`.
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}

/// There was a problem parsing `str` as [`Effects`][super::Effects].
#[derive(Debug, Error)]
#[error(transparent)]
pub struct ParseEffectsError(#[from] ParseIntError);

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseSampleIndexError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}

/// Error for when there was a problem setting / parsing the volume.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum VolumeError {
    /// There was a problem parsing the `str` as [`Volume`][super::Volume].
    #[error(transparent)]
    ParseVolumeError(#[from] ParseIntError),
}
