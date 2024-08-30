pub mod error;
pub mod types;

use std::str::FromStr;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::helper::*;
use crate::OsuFile;

pub use error::*;
pub use types::*;

use super::Error;
use super::Integer;
use super::Position;
use super::Version;
use super::VersionedDefault;
use super::VersionedFromStr;
use super::VersionedToString;
use super::VersionedTryFrom;

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct HitObjects(pub Vec<HitObject>);

impl VersionedFromStr for HitObjects {
    type Err = Error<ParseError>;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let mut hitobjects = Vec::new();

        for (line_index, s) in s.lines().enumerate() {
            if s.trim().is_empty() {
                continue;
            }

            hitobjects.push(Error::new_from_result_into(
                HitObject::from_str(s, version).map(|v| v.unwrap()),
                line_index,
            )?);
        }

        Ok(Some(HitObjects(hitobjects)))
    }
}

impl VersionedToString for HitObjects {
    fn to_string(&self, version: Version) -> Option<String> {
        Some(
            self.0
                .iter()
                .filter_map(|o| o.to_string(version))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

impl VersionedDefault for HitObjects {
    fn default(_: Version) -> Option<Self> {
        Some(HitObjects(Vec::new()))
    }
}

/// A struct that represents a hitobject.
///
/// All hitobjects will have the properties: `x`, `y`, `time`, `type`, `hitsound`, `hitsample`.
///
/// The `type` property is a `u8` integer with each bit flags containing some information, which are split into the functions and enums:
/// [hitobject_type][Self::obj_params], [new_combo][Self::new_combo], [combo_skip_count][Self::combo_skip_count]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub struct HitObject {
    /// The position of the hitobject.
    pub position: Position,
    /// The time when the object is to be hit, in milliseconds from the beginning of the beatmap's audio.
    pub time: u32,
    /// The hitobject parameters.
    /// Each hitobject contains different parameters.
    /// Also is used to know which hitobject type this is.
    pub obj_params: HitObjectParams,
    /// If the hitobject is a new combo.
    pub new_combo: bool,
    /// A 3-bit integer specifying how many combo colours to skip, if this object starts a new combo.
    pub combo_skip_count: ComboSkipCount,
    /// The [hitsound][HitSound] property of the hitobject.
    pub hitsound: HitSound,
    /// The [hitsample][HitSample] property of the hitobject.
    pub hitsample: Option<HitSample>,
}

impl HitObject {
    fn type_to_string(&self) -> String {
        let mut bit_flag: u8 = 0;

        bit_flag |= match self.obj_params {
            HitObjectParams::HitCircle => 1,
            HitObjectParams::Slider { .. } => 2,
            HitObjectParams::Spinner { .. } => 8,
            HitObjectParams::OsuManiaHold { .. } => 128,
        };

        if self.new_combo {
            bit_flag |= 4;
        }

        // 3 bit value from 4th ~ 6th bits
        bit_flag |= self.combo_skip_count.get() << 4;

        bit_flag.to_string()
    }

    pub fn hitcircle_default() -> Self {
        Self {
            position: Default::default(),
            time: Default::default(),
            obj_params: HitObjectParams::HitCircle,
            new_combo: Default::default(),
            combo_skip_count: Default::default(),
            hitsound: Default::default(),
            hitsample: Default::default(),
        }
    }

    pub fn spinner_default() -> Self {
        Self {
            position: Default::default(),
            time: Default::default(),
            obj_params: HitObjectParams::Spinner {
                end_time: Default::default(),
            },
            new_combo: Default::default(),
            combo_skip_count: Default::default(),
            hitsound: Default::default(),
            hitsample: Default::default(),
        }
    }

    pub fn osu_mania_hold_default() -> Self {
        Self {
            position: Position {
                x: dec!(0).into(),
                ..Default::default()
            },
            time: Default::default(),
            obj_params: HitObjectParams::OsuManiaHold {
                end_time: Default::default(),
            },
            new_combo: Default::default(),
            combo_skip_count: Default::default(),
            hitsound: Default::default(),
            hitsample: Default::default(),
        }
    }
}

impl VersionedFromStr for HitObject {
    type Err = ParseHitObjectError;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let split: Vec<&str> = s.split(',').collect();

        let position = Position {
            x: split[0]
                .parse::<Decimal>()
                .map_err(|_| ParseHitObjectError::InvalidX)?,
            y: split[1]
                .parse::<Decimal>()
                .map_err(|_| ParseHitObjectError::InvalidY)?,
        };

        let time = split[2]
            .parse::<u32>()
            .map(|t| add_old_version_time_offset(t, version))
            .map_err(|_| ParseHitObjectError::InvalidTime)?;

        let obj_type_number = split[3].parse::<HitObjectTypeNumber>()?;
        let hitsound = HitSound::from_str(split[4], version)?.unwrap();

        match obj_type_number.obj_type {
            // hitcircle syntax:
            // x,y,time,type,hitsound(,[[hitsample|0:0:0:0:]|''])
            HitObjectType::HitCircle => Ok(Some(Self {
                position,
                time,
                obj_params: HitObjectParams::HitCircle,
                new_combo: obj_type_number.new_combo,
                combo_skip_count: obj_type_number.combo_skip_count,
                hitsound,
                hitsample: if split.len() == 6 {
                    Some(HitSample::from_str(split[5], version)?.unwrap())
                } else {
                    None
                }
            })),
            // slider syntax:
            // x,y,time,type,hitSound,curveType|curvePoints,slides,length,edgeSounds,edgeSets,hitSample
            // 0 1 2    3    4        5                     6      7      8          9        10
            HitObjectType::Slider => {
                if split.len() != 11 {
                    return Err(ParseHitObjectError::InvalidLength)
                }

                let mut subsplit = split[5].split('|');
                let curve_type = subsplit
                    .next()
                    .ok_or_else(|| ParseHitObjectError::InvalidCurveType)?;
                let curve_type = CurveType
                    ::from_str(curve_type, version)
                    .map_err(|_| ParseHitObjectError::InvalidCurveType)?
                    .unwrap();

                let params = SlideParams {
                    curve_type,
                    curve_points: subsplit
                        .map(|p| CurvePoint::from_str(p, version))
                        .collect::<Result<Vec<Option<CurvePoint>>, ParseCurvePointError>>()?
                        .iter()
                        .map(|p| p.unwrap())
                        .collect::<Vec<CurvePoint>>(),
                    slides: split[6]
                        .parse::<Integer>()
                        .map_err(|_| ParseHitObjectError::InvalidSlidesCount)?,
                    length: split[7]
                        .parse::<Decimal>()
                        .map_err(|_| ParseHitObjectError::InvalidLength)?,
                    edge_sounds: split[8]
                        .split('|')
                        .map(|s| HitSound::from_str(s, version))
                        .collect::<Result<Vec<Option<HitSound>>, ParseHitSoundError>>()?
                        .iter()
                        .map(|s| s.unwrap())
                        .collect::<Vec<HitSound>>(),
                    edge_sets: split[9]
                        .split('|')
                        .map(|s| EdgeSet::from_str(s, version))
                        .collect::<Result<Vec<Option<EdgeSet>>, ParseColonSetError>>()?
                        .iter()
                        .map(|s| s.unwrap())
                        .collect::<Vec<EdgeSet>>()
                };
                Ok(Some(Self {
                    position,
                    time,
                    obj_params: HitObjectParams::Slider(params),
                    new_combo: obj_type_number.new_combo,
                    combo_skip_count: obj_type_number.combo_skip_count,
                    hitsound,
                    hitsample: Some(HitSample::from_str(split[10], version)?.unwrap())
                }))
            },
            HitObjectType::Spinner => { },
            HitObjectType::OsuManiaHold => { }
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum HitObjectParams {
    HitCircle,
    Slider(SlideParams),
    Spinner { end_time: u32 },
    OsuManiaHold { end_time: u32 },
}

pub enum HitObjectType {
    HitCircle,
    Slider,
    Spinner,
    OsuManiaHold
}

pub struct HitObjectTypeNumber {
    number: u8,
    new_combo: bool,
    combo_skip_count: ComboSkipCount,
    obj_type: HitObjectType
}

impl FromStr for HitObjectTypeNumber {
    type Err = ParseHitObjectTypeNumberError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let number = value.parse::<u8>()?;
        
        let hitcircle = number >> 0 & 1;
        let slider = number >> 1 & 1;
        let new_combo = (number >> 2 & 1) != 0;
        let spinner = number >> 3 & 1;
        let combo_skip_count = ComboSkipCount::try_from(number)?;
        let mania_hold_note = number >> 7 & 1;

        // Only one object type flag can be active
        if hitcircle + slider + spinner + mania_hold_note != 1 {
            return Err(ParseHitObjectTypeNumberError::InvalidObjType);
        }

        Ok(Self {
            number,
            new_combo,
            combo_skip_count,
            obj_type: match true {
                hitcircle => HitObjectType::HitCircle,
                slider => HitObjectType::Slider,
                spinner => HitObjectType::Spinner,
                mania_hold_note => HitObjectType::OsuManiaHold,
            }
        })
    }
}


#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SlideParams {
    pub curve_type: CurveType,
    pub curve_points: Vec<CurvePoint>,
    pub slides: Integer,
    pub length: Decimal,
    pub edge_sounds: Vec<HitSound>,
    pub edge_sets: Vec<EdgeSet>,
}