//! Module defining `error` types that's used for the `hitobject` related modules.

use std::num::ParseIntError;

use strum_macros::{EnumString, IntoStaticStr};
use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct ParseError(#[from] ParseHitObjectError);

#[derive(Debug, Error)]
#[error("Expected combo skip count to be 3 bits")]
pub struct ComboSkipCountTooHigh;

#[derive(Debug, Error, IntoStaticStr, EnumString)]
#[non_exhaustive]
/// Error used when there was a problem parsing a `str` into a `ColonSet`.
pub enum ParseColonSetError {
    /// When the length of the set is not 2.
    #[error("The set is not two elements long.")]
    InvalidLength,
    #[error("Invalid `normal_set` or `addition_set` value")]
    InvalidSet(#[from] ParseSampleSetError),
}


#[derive(Debug, Error, IntoStaticStr, EnumString)]
#[non_exhaustive]
pub enum ParseCurvePointError {
    /// When the length of the set is not 2.
    #[error("The set is not two elements long.")]
    InvalidLength,
    #[error("Invalid `x` value")]
    InvalidX,
    #[error("Invalid `y` value")]
    InvalidY,
}

#[derive(Debug, Error, IntoStaticStr)]
#[non_exhaustive]
/// Error used when there was a problem parsing a `str` into a [`HitObject`][super::HitObject].
pub enum ParseHitObjectError {
    /// Invalid `x` value.
    #[error("Invalid `x` value")]
    InvalidX,
    /// Invalid `y` value.
    #[error("Invalid `y` value")]
    InvalidY,
    /// Missing `time` field.
    #[error("Missing `time` field")]
    MissingTime,
    /// Invalid `time` value.
    #[error("Invalid `time` value")]
    InvalidTime,
    /// Invalid `curve_type` value.
    #[error("Invalid `curve_type` value")]
    InvalidCurveType,
    /// Invalid `curve_point` value.
    #[error(transparent)]
    InvalidCurvePoint(#[from] ParseCurvePointError),
    /// Invalid `edge_sound` value.
    #[error("Invalid `edge_sound` value")]
    InvalidEdgeSound,
    /// Invalid `edge_set` value.
    #[error("Invalid `edge_set` value")]
    InvalidEdgeSet(#[from] ParseColonSetError),
    /// Invalid `slides_count` value.
    #[error("Invalid `slides_count` value")]
    InvalidSlidesCount,
    /// Invalid `length` value.
    #[error("Invalid `length` value")]
    InvalidLength,
    /// Invalid `end_time` value.
    #[error("Invalid `end_time` value")]
    InvalidEndTime,
    /// Unknown object type.
    #[error("Unknown object type")]
    UnknownObjType,
    /// Passthroughs
    #[error(transparent)]
    InvalidComboSkipCount(#[from] ComboSkipCountTooHigh),
    #[error(transparent)]
    InvalidHitSample(#[from] ParseHitSampleError),
    #[error(transparent)]
    InvalidHitSound(#[from] ParseHitSoundError),
    #[error(transparent)]
    InvalidHitObjectTypeNumber(#[from] ParseHitObjectTypeNumberError)
}

#[derive(Debug, Error, IntoStaticStr)]
#[non_exhaustive]
pub enum ParseHitObjectTypeNumberError {
    /// Invalid `obj_type` value.
    #[error("Invalid `obj_type` value")]
    InvalidObjType,
    #[error("There was a problem parsing the `str` into an integer first")]
    ParseValueError(#[from] ParseIntError),
    #[error(transparent)]
    ComboSkipCountTooHigh(#[from] ComboSkipCountTooHigh)
}

#[derive(Debug, Error, EnumString, IntoStaticStr)]
#[non_exhaustive]
/// Error used when there was a problem parsing a `str` into a [`hitsample`][super::types::HitSample].
pub enum ParseHitSampleError {
    #[error("The set is not at least four elements long.")]
    InvalidLength,
    #[error("Invalid `normal_set` value")]
    InvalidNormalSet,
    #[error("Invalid `addition_set` value")]
    InvalidAdditionSet,
    #[error("Invalid `index` value")]
    InvalidIndex,
    #[error("Invalid `volume` value")]
    InvalidVolume,
}

#[derive(Debug, Error)]
#[non_exhaustive]
/// Error used when there was a problem parsing a `str` into a [`sampleset`][super::types::SampleSet].
pub enum ParseSampleSetError {
    /// There was a problem parsing a `str` as an integer.
    #[error("There was a problem parsing the `str` into an integer first")]
    ParseValueError(#[from] ParseIntError),
}

#[derive(Debug, Error)]
#[non_exhaustive]
/// Error used when the user tried to set [`volume`][super::types::Volume]'s field as something invalid.
pub enum VolumeSetError {
    #[error("The volume was too high, expected to be in range 1 ~ 100")]
    VolumeTooHigh,
    /// The volume has a value `0`, which is "invalid".
    /// In the osu file documentation, the volume of 0 means the `timingpoint`'s volume is used instead.
    /// I handle that special case differently to make it more clear to the user what's going on.
    #[error("The volume was attempted to set to 0, expected to be in range 1 ~ 100")]
    VolumeTooLow,
}

#[derive(Debug, Error)]
#[non_exhaustive]
/// Error used when there was a problem parsing a `volume` from a `str`.
pub enum ParseVolumeError {
    #[error(transparent)]
    VolumeSetError(#[from] VolumeSetError),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseCurveTypeError {
    #[error("Unknown `CurveType` variant")]
    UnknownVariant,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseHitSoundError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}