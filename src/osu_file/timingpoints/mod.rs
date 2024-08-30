pub mod error;
pub mod types;

use rust_decimal::{prelude::ToPrimitive, Decimal};
use rust_decimal_macros::dec;

use super::{
    Error, Integer, Version, VersionedDefault, VersionedFrom, VersionedFromStr, VersionedToString,
};

pub use error::*;
pub use types::*;

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct TimingPoints(pub Vec<TimingPoint>);

impl VersionedFromStr for TimingPoints {
    type Err = Error<ParseError>;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let mut timing_points = Vec::new();

        for (line_index, s) in s.lines().enumerate() {
            if s.trim().is_empty() {
                continue;
            }

            timing_points.push(Error::new_from_result_into(
                TimingPoint::from_str(s, version),
                line_index,
            )?);
        }

        if let Some(s) = timing_points.get(0) {
            if s.is_some() {
                Ok(Some(TimingPoints(
                    timing_points
                        .into_iter()
                        .map(|v| v.unwrap())
                        .collect::<Vec<_>>(),
                )))
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(TimingPoints(Vec::new())))
        }
    }
}

impl VersionedDefault for TimingPoints {
    fn default(_: Version) -> Option<Self> {
        Some(TimingPoints(Vec::new()))
    }
}

/// Struct representing a timing point.
/// Each timing point influences a specified portion of the map, commonly called a `timing section`.
/// The .osu file format requires these to be sorted in chronological order.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct TimingPoint {
    // for some reason decimal is parsed anyway in the beatmap???
    pub time: Integer,
    pub beat_length: Decimal,
    pub meter: Integer,
    pub sample_set: SampleSet,
    pub sample_index: SampleIndex,
    pub volume: Volume,
    pub uninherited: bool,
    pub effects: Option<Effects>,
}

impl TimingPoint {
    /// Converts beat duration in milliseconds to BPM.
    pub fn beat_duration_ms_to_bpm(
        beat_duration_ms: rust_decimal::Decimal,
    ) -> rust_decimal::Decimal {
        rust_decimal::Decimal::ONE / beat_duration_ms * dec!(60000)
    }

    /// Converts BPM to beat duration in milliseconds.
    pub fn bpm_to_beat_duration_ms(bpm: rust_decimal::Decimal) -> rust_decimal::Decimal {
        rust_decimal::Decimal::ONE / (bpm / dec!(60000))
    }

    /// New instance of `TimingPoint` that is inherited.
    pub fn new_inherited(
        time: Integer,
        slider_velocity_multiplier: rust_decimal::Decimal,
        meter: Integer,
        sample_set: SampleSet,
        sample_index: SampleIndex,
        volume: Volume,
        effects: Effects,
    ) -> Self {
        let beat_length = (rust_decimal::Decimal::ONE / slider_velocity_multiplier) * dec!(-100);

        Self {
            time: time.into(),
            beat_length: beat_length.into(),
            meter,
            sample_set,
            sample_index,
            volume,
            uninherited: false,
            effects: Some(effects),
        }
    }

    /// New instance of `TimingPoint` that is uninherited.
    pub fn new_uninherited(
        time: Integer,
        beat_duration_ms: Decimal,
        meter: Integer,
        sample_set: SampleSet,
        sample_index: SampleIndex,
        volume: Volume,
        effects: Effects,
    ) -> Self {
        Self {
            time: time.into(),
            beat_length: beat_duration_ms,
            meter,
            sample_set,
            sample_index,
            volume,
            uninherited: true,
            effects: Some(effects),
        }
    }

    /// Calculates BPM using the `beatLength` field when unherited.
    /// - Returns `None` if the timing point is inherited or `beat_length` isn't a valid decimal.
    pub fn calc_bpm(&self) -> Option<rust_decimal::Decimal> {
        if self.uninherited {
            Some(Self::beat_duration_ms_to_bpm(self.beat_length))
        } else {
            None
        }
    }
    /// Calculates the slider velocity multiplier when the timing point is inherited.
    /// - Returns `None` if the timing point is uninherited or `beat_length` isn't a valid decimal.
    pub fn calc_slider_velocity_multiplier(&self) -> Option<rust_decimal::Decimal> {
        if self.uninherited {
            None
        } else {
            Some(rust_decimal::Decimal::ONE / (self.beat_length / dec!(-100)))
        }
    }
}

const OLD_VERSION_TIME_OFFSET: rust_decimal::Decimal = dec!(24);

impl VersionedFromStr for TimingPoint {
    type Err = ParseTimingPointError;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let meter_fallback = 4;
        let sample_set_fallback = SampleSet::Normal;
        let sample_index_fallback = <SampleIndex as VersionedFrom<u32>>::from(1, version).unwrap();
        let volume_fallback = <Volume as VersionedFrom<Integer>>::from(100, version).unwrap();

        // make this simple bruh
        let split_by_comma: Vec<&str> = s.split(",").collect();

        if split_by_comma.len() != 8 {
            return Err(ParseTimingPointError::InvalidFieldCount);
        }

        Ok(Some(TimingPoint {
            time: {
                let t = split_by_comma[0]
                    .parse::<Integer>()
                    .map_err(|_| { ParseTimingPointError::InvalidTime })?;

                if (3..=4).contains(&version) {
                    t + OLD_VERSION_TIME_OFFSET.to_i32().unwrap()
                } else {
                    t
                }
            },
            beat_length: split_by_comma[1]
                .parse::<Decimal>()
                .map_err(|_| { ParseTimingPointError::InvalidBeatLength })?,
            meter: split_by_comma[2]
                .parse::<Integer>()
                .map_err(|_| { ParseTimingPointError::InvalidMeter })?,
            sample_set: SampleSet
                ::from_str(split_by_comma[3], version)
                .map_err(|_| { ParseTimingPointError::InvalidSampleSet })?
                .unwrap(),
            sample_index: SampleIndex
                ::from_str(split_by_comma[4], version)
                .map_err(|_| { ParseTimingPointError::InvalidSampleIndex })?
                .unwrap(),
            volume: Volume
                ::from_str(split_by_comma[5], version)
                .map_err(|_| { ParseTimingPointError::InvalidVolume })?
                .unwrap(),
            uninherited: match split_by_comma[6] {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(ParseTimingPointError::InvalidUninherited)
            }?,
            effects: Effects
                ::from_str(split_by_comma[7], version)
                .map_err(|_| { ParseTimingPointError::InvalidVolume })?
        }))
    }
}