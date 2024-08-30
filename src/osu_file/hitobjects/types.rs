use std::num::{NonZeroUsize, ParseIntError};
use rust_decimal::Decimal;

use crate::{
    helper::nth_bit_state_i64,
    osu_file::*
};

use super::error::*;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComboSkipCount(u8);

impl ComboSkipCount {
    pub fn new(count: u8, version: Version) -> Result<Option<Self>, ComboSkipCountTooHigh> {
        <ComboSkipCount as VersionedTryFrom<u8>>::try_from(count, version)
    }

    pub fn get(&self) -> u8 {
        self.0
    }

    pub fn set(&mut self, count: u8, version: Version) -> Result<(), ComboSkipCountTooHigh> {
        let new_self = <ComboSkipCount as VersionedTryFrom<u8>>::try_from(count, version)?.unwrap();
        *self = new_self;
        Ok(())
    }
}

impl TryFrom<u8> for ComboSkipCount {
    type Error = ComboSkipCountTooHigh;

    fn try_from(type_number: u8) -> Result<Self, Self::Error> {
        let value = type_number >> 4 & 0b111;

        // limit to 3 bits
        if value > 0b111 {
            Err(ComboSkipCountTooHigh)
        } else {
            Ok(Self(value))
        }
    }
}

impl VersionedFrom<ComboSkipCount> for u8 {
    fn from(count: ComboSkipCount, _: Version) -> Option<Self> {
        Some(count.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Sample sets used for the `edgeSounds`.
pub struct EdgeSet {
    /// Sample set of the normal sound.
    pub normal_set: SampleSet,
    /// Sample set of the whistle, finish, and clap sounds.
    pub addition_set: SampleSet,
}

impl VersionedFromStr for EdgeSet {
    type Err = ParseColonSetError;

    fn from_str(s: &str, version: Version) -> Result<Option<Self>, Self::Err> {
        let split: Vec<&str> = s.split(':').collect();
        if split.len() != 2 {
            return Err(ParseColonSetError::InvalidLength);
        }

        Ok(Some(Self {
            normal_set: split[0].parse::<SampleSet>()?,
            addition_set: split[1].parse::<SampleSet>()?
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Anchor point used to construct the [`slider`][super::SlideParams].
pub struct CurvePoint(pub Position);

impl VersionedFromStr for CurvePoint {
    type Err = ParseCurvePointError;

    fn from_str(s: &str, _: Version) -> Result<Option<Self>, Self::Err> {
        let split: Vec<&str> = s.split(':').collect();
        if split.len() != 2 {
            return Err(ParseCurvePointError::InvalidLength);
        }

        Ok(Some(Self(Position {
            x: split[0]
                .parse::<Decimal>()
                .map_err(|_| { ParseCurvePointError::InvalidX })?,
            y: split[1]
                .parse::<Decimal>()
                .map_err(|_| { ParseCurvePointError::InvalidY })?
        })))
    }
}

impl VersionedToString for CurvePoint {
    fn to_string(&self, _: Version) -> Option<String> {
        Some(format!("{}:{}", self.0.x, self.0.y))
    }
}

/// Used for `normal_set` and `addition_set` for the `[hitobject]`[super::HitObject].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SampleSet {
    /// No custom sample set.
    NoCustomSampleSet,
    /// Normal set.
    NormalSet,
    /// Soft set.
    SoftSet,
    /// Drum set.
    DrumSet,
    /// There is a case where some maps have an extremely high value for this.
    Other(usize),
}

impl From<SampleSet> for usize {
    fn from(set: SampleSet) -> Self {
        match set {
            SampleSet::NoCustomSampleSet => 0,
            SampleSet::NormalSet => 1,
            SampleSet::SoftSet => 2,
            SampleSet::DrumSet => 3,
            SampleSet::Other(other) => other,
        }
    }
}

impl VersionedDefault for SampleSet {
    fn default(_: Version) -> Option<Self> {
        Some(Self::NoCustomSampleSet)
    }
}

impl FromStr for SampleSet {
    type Err = ParseSampleSetError;

    fn from_str(s: &str) -> Result<SampleSet, ParseSampleSetError> {
        let s = s.parse()?;

        Ok(match s {
            0 => Self::NoCustomSampleSet,
            1 => Self::NormalSet,
            2 => Self::SoftSet,
            3 => Self::DrumSet,
            _ => Self::Other(s),
        })
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
/// Volume of the sample from `1` to `100`. If [volume][Self::volume] returns `None`, the timing point's volume will be used instead.
pub struct Volume(Option<u8>);

impl VersionedDefault for Volume {
    fn default(_: Version) -> Option<Self> {
        Some(Volume(None))
    }
}

impl VersionedFrom<Volume> for Integer {
    fn from(volume: Volume, _: Version) -> Option<Self> {
        let volume = match volume.0 {
            Some(volume) => volume as Integer,
            None => 0,
        };

        Some(volume)
    }
}

impl Volume {
    /// Creates a new instance of `Volume`.
    /// - Requires the `volume` to be in range of `1` ~ `100`.
    /// - Setting the `volume` to be `None` will use the timingpoint's volume instead.
    pub fn new(volume: Option<u8>) -> Result<Volume, VolumeSetError> {
        if let Some(volume) = volume {
            if volume == 0 {
                Err(VolumeSetError::VolumeTooLow)
            } else if volume > 100 {
                Err(VolumeSetError::VolumeTooHigh)
            } else {
                Ok(Volume(Some(volume)))
            }
        } else {
            Ok(Volume(volume))
        }
    }

    /// Returns the `volume`.
    pub fn volume(&self) -> Option<u8> {
        self.0
    }

    /// Sets the `volume`.
    /// Requires to be in the boundary of `1` ~ `100`
    pub fn set_volume(&mut self, volume: u8) -> Result<(), VolumeSetError> {
        if volume == 0 {
            Err(VolumeSetError::VolumeTooLow)
        } else if volume > 100 {
            Err(VolumeSetError::VolumeTooHigh)
        } else {
            self.0 = Some(volume);
            Ok(())
        }
    }

    /// Returns `true` if the timingpoint volume is used instead.
    pub fn use_timing_point_volume(&self) -> bool {
        self.0.is_none()
    }

    /// Sets if to use the timingpoint volume instead.
    pub fn set_use_timing_point_volume(&mut self) {
        self.0 = None;
    }
}

impl VersionedFromStr for Volume {
    type Err = ParseVolumeError;

    fn from_str(s: &str, _: Version) -> Result<Option<Self>, Self::Err> {
        let volume = s.parse::<u8>()?;

        let volume = if volume == 0 { None } else { Some(volume) };

        Ok(Volume::new(volume).map(Some)?)
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
/// Flags that determine which sounds will play when the object is hit.
/// # Possible sounds
/// [`normal`][Self::normal] [`whistle`][Self::whistle] [`finish`][Self::finish] [`clap`][Self::clap]
/// - If no flags are set, it will default to using the `normal` sound.
/// - In every mode except osu!mania, the `LayeredHitSounds` skin property forces the `normal` sound to be used, and ignores those flags.
pub struct HitSound {
    normal: bool,
    whistle: bool,
    finish: bool,
    clap: bool,
}

impl VersionedToString for HitSound {
    fn to_string(&self, _: Version) -> Option<String> {
        let mut bit_mask = 0;

        if self.normal {
            bit_mask |= 1;
        }
        if self.whistle {
            bit_mask |= 2;
        }
        if self.finish {
            bit_mask |= 4;
        }
        if self.clap {
            bit_mask |= 8;
        }

        Some(bit_mask.to_string())
    }
}

impl HitSound {
    pub fn new(normal: bool, whistle: bool, finish: bool, clap: bool) -> Self {
        Self {
            normal,
            whistle,
            finish,
            clap,
        }
    }

    /// Returns the `normal` flag. Also will return `true` if no sound flags are set.
    pub fn normal(&self) -> bool {
        if !self.normal && !self.whistle && !self.finish && !self.clap {
            true
        } else {
            self.normal
        }
    }
    /// Returns the `whistle` flag.
    pub fn whistle(&self) -> bool {
        self.whistle
    }
    /// Returns the `finish` flag.
    pub fn finish(&self) -> bool {
        self.finish
    }
    /// Returns the `clap` flag.
    pub fn clap(&self) -> bool {
        self.clap
    }

    /// Sets the `normal` flag.
    pub fn set_normal(&mut self, normal: bool) {
        self.normal = normal;
    }
    /// Sets the `whistle` flag.
    pub fn set_whistle(&mut self, whistle: bool) {
        self.whistle = whistle;
    }
    /// Sets the `finish` flag.
    pub fn set_finish(&mut self, finish: bool) {
        self.finish = finish;
    }
    /// Sets the `clap` flag.
    pub fn set_clap(&mut self, clap: bool) {
        self.clap = clap;
    }
}

impl VersionedFromStr for HitSound {
    type Err = ParseHitSoundError;

    fn from_str(s: &str, version: Version) -> Result<Option<Self>, Self::Err> {
        Ok(<HitSound as VersionedFrom<u8>>::from(s.parse()?, version))
    }
}

impl VersionedFrom<u8> for HitSound {
    fn from(value: u8, _: Version) -> Option<Self> {
        let normal = nth_bit_state_i64(value as i64, 0);
        let whistle = nth_bit_state_i64(value as i64, 1);
        let finish = nth_bit_state_i64(value as i64, 2);
        let clap = nth_bit_state_i64(value as i64, 3);

        Some(Self {
            normal,
            whistle,
            finish,
            clap,
        })
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Hash)]
#[non_exhaustive]
/// Type of curve used to construct the [`Slider`][super::SlideParams].
pub enum CurveType {
    /// Bézier curve.
    Bezier,
    /// Centripetal catmull-rom curve.
    Centripetal,
    /// Linear curve.
    Linear,
    /// Perfect circle curve.
    PerfectCircle,
}

impl VersionedFromStr for CurveType {
    type Err = ParseCurveTypeError;

    fn from_str(s: &str, _: Version) -> std::result::Result<Option<Self>, Self::Err> {
        match s {
            "B" => Ok(Some(CurveType::Bezier)),
            "C" => Ok(Some(CurveType::Centripetal)),
            "L" => Ok(Some(CurveType::Linear)),
            "P" => Ok(Some(CurveType::PerfectCircle)),
            _ => Err(ParseCurveTypeError::UnknownVariant),
        }
    }
}

impl VersionedToString for CurveType {
    fn to_string(&self, _: Version) -> Option<String> {
        let curve = match self {
            CurveType::Bezier => "B",
            CurveType::Centripetal => "C",
            CurveType::Linear => "L",
            CurveType::PerfectCircle => "P",
        };

        Some(curve.to_string())
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum SampleIndex {
    TimingPointSampleIndex,
    Index(NonZeroUsize),
}

impl VersionedDefault for SampleIndex {
    fn default(_: Version) -> Option<Self> {
        Some(Self::TimingPointSampleIndex)
    }
}

impl VersionedFrom<usize> for SampleIndex {
    fn from(index: usize, _: Version) -> Option<Self> {
        let index = if index == 0 {
            Self::TimingPointSampleIndex
        } else {
            Self::Index(NonZeroUsize::new(index).unwrap())
        };

        Some(index)
    }
}

impl VersionedFrom<SampleIndex> for usize {
    fn from(index: SampleIndex, _: Version) -> Option<Self> {
        let index = match index {
            SampleIndex::TimingPointSampleIndex => 0,
            SampleIndex::Index(index) => index.get(),
        };

        Some(index)
    }
}

impl VersionedToString for SampleIndex {
    fn to_string(&self, version: Version) -> Option<String> {
        <usize as VersionedFrom<SampleIndex>>::from(*self, version).map(|i| i.to_string())
    }
}

impl VersionedFromStr for SampleIndex {
    type Err = ParseIntError;

    fn from_str(s: &str, version: Version) -> Result<Option<Self>, Self::Err> {
        Ok(<SampleIndex as VersionedFrom<usize>>::from(
            s.parse::<usize>()?,
            version,
        ))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// Information about which samples are played when the object is hit.
/// It is closely related to [`hitSound`][HitSound].
pub struct HitSample {
    pub normal_set: SampleSet,
    pub addition_set: SampleSet,
    pub index: SampleIndex,
    pub volume: Volume,
    pub filename: Option<String>,
}

impl VersionedFromStr for HitSample {
    type Err = ParseHitSampleError;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let split: Vec<&str> = s.split(':').collect();
        
        if split.len() < 4 {
            return Err(ParseHitSampleError::InvalidLength);
        }

        Ok(Some(Self {
            normal_set: SampleSet
                ::from_str(split[0], version)
                .map_err(|_| { ParseHitSampleError::InvalidNormalSet })?
                .unwrap(),
            addition_set: SampleSet
                ::from_str(split[1], version)
                .map_err(|_| { ParseHitSampleError::InvalidAdditionSet })?
                .unwrap(),
            index: SampleIndex
                ::from_str(split[2], version)
                .map_err(|_| { ParseHitSampleError::InvalidIndex })?
                .unwrap(),
            volume: Volume
                ::from_str(split[3], version)
                .map_err(|_| { ParseHitSampleError::InvalidVolume })?
                .unwrap(),
            filename: if split.len() == 5 {
                Some(String::from_str(split[4]).unwrap())
            } else {
                None
            }
        }))
    }
}

impl VersionedToString for HitSample {
    fn to_string(&self, version: Version) -> Option<String> {
        let volume: Integer = <i32 as VersionedFrom<Volume>>::from(self.volume, version).unwrap();
        let filename = &self.filename.unwrap_or_default();

        match version {
            MIN_VERSION..=9 => None,
            10..=11 => Some(format!(
                "{}:{}:{}",
                self.normal_set.to_string(version).unwrap(),
                self.addition_set.to_string(version).unwrap(),
                self.index.to_string(version).unwrap()
            )),
            _ => Some(format!(
                "{}:{}:{}:{volume}:{filename}",
                self.normal_set.to_string(version).unwrap(),
                self.addition_set.to_string(version).unwrap(),
                self.index.to_string(version).unwrap()
            )),
        }
    }
}

impl VersionedDefault for HitSample {
    fn default(version: Version) -> Option<Self> {
        Some(HitSample {
            normal_set: SampleSet::default(version).unwrap(),
            addition_set: SampleSet::default(version).unwrap(),
            index: SampleIndex::default(version).unwrap(),
            volume: Volume::default(version).unwrap(),
            filename: None,
        })
    }
}
