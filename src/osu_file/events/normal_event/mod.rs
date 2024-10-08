use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::eof,
    error::context,
    sequence::{preceded, tuple},
    Parser,
};
use rust_decimal::Decimal;

use crate::{
    osu_file::{FilePath, Integer, Position, Version, VersionedFromStr, VersionedToString},
    parsers::{
        comma, comma_field, comma_field_type, comma_field_versioned_type, consume_rest_type,
        consume_rest_versioned_type,
    },
};

pub use error::*;

use self::types::{LayerLegacy, OriginTypeLegacy};

use super::{storyboard::cmds::Command, EventWithCommands, Volume, OLD_VERSION_TIME_OFFSET};

pub mod error;
mod parsers;
pub mod types;
use parsers::*;

fn position_str(position: &Option<Position>) -> String {
    match position {
        Some(position) => format!(",{},{}", position.x, position.y),
        None => String::new(),
    }
}

fn time_to_string(time: Integer, version: Version) -> String {
    let time = if (3..=4).contains(&version) {
        time - OLD_VERSION_TIME_OFFSET
    } else {
        time
    };

    time.to_string()
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Background {
    pub start_time: Integer,
    pub file_name: FilePath,
    pub position: Option<Position>,
    pub commands: Vec<Command>,
}

pub const BACKGROUND_HEADER: &str = "0";

impl VersionedFromStr for Background {
    type Err = ParseBackgroundError;

    fn from_str(s: &str, _: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let (_, (start_time, (filename, position))) = preceded(
            tuple((
                context(
                    ParseBackgroundError::WrongEventType.into(),
                    tag(BACKGROUND_HEADER),
                ),
                context(ParseBackgroundError::MissingStartTime.into(), comma()),
            )),
            tuple((
                context(
                    ParseBackgroundError::InvalidStartTime.into(),
                    comma_field_type(),
                ),
                preceded(
                    context(ParseBackgroundError::MissingFileName.into(), comma()),
                    file_name_and_position(
                        ParseBackgroundError::MissingX.into(),
                        ParseBackgroundError::InvalidX.into(),
                        ParseBackgroundError::MissingY.into(),
                        ParseBackgroundError::InvalidY.into(),
                    ),
                ),
            )),
        )(s)?;

        Ok(Some(Background {
            start_time,
            file_name: filename,
            position,
            commands: Vec::new(),
        }))
    }
}

impl VersionedToString for Background {
    fn to_string(&self, version: Version) -> Option<String> {
        self.to_string_variables(version, &[])
    }
}

impl EventWithCommands for Background {
    fn commands(&self) -> &[Command] {
        &self.commands
    }

    fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    fn to_string_cmd(&self, version: Version) -> Option<String> {
        Some(format!(
            "{BACKGROUND_HEADER},{},{}{}",
            self.start_time,
            self.file_name.to_string(version).unwrap(),
            position_str(&self.position),
        ))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Video {
    pub start_time: Integer,
    pub file_name: FilePath,
    pub position: Option<Position>,
    pub commands: Vec<Command>,
    short_hand: bool,
}

impl Video {
    pub fn new(start_time: Integer, file_name: FilePath, position: Option<Position>) -> Self {
        Self {
            commands: Vec::new(),
            start_time,
            file_name,
            position,
            short_hand: true,
        }
    }
}

pub const VIDEO_HEADER: &str = "1";
pub const VIDEO_HEADER_LONG: &str = "Video";

impl VersionedFromStr for Video {
    type Err = ParseVideoError;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let (_, (short_hand, start_time, (file_name, position))) = tuple((
            alt((
                tag(VIDEO_HEADER).map(|_| true),
                context(
                    ParseVideoError::WrongEventType.into(),
                    tag(VIDEO_HEADER_LONG).map(|_| false),
                ),
            )),
            preceded(
                context(ParseVideoError::MissingStartTime.into(), comma()),
                start_time_offset(ParseVideoError::InvalidStartTime.into(), version),
            ),
            preceded(
                context(ParseVideoError::MissingFileName.into(), comma()),
                file_name_and_position(
                    ParseVideoError::MissingX.into(),
                    ParseVideoError::InvalidX.into(),
                    ParseVideoError::MissingY.into(),
                    ParseVideoError::InvalidY.into(),
                ),
            ),
        ))(s)?;

        Ok(Some(Video {
            commands: Vec::new(),
            start_time,
            file_name,
            position,
            short_hand,
        }))
    }
}

impl VersionedToString for Video {
    fn to_string(&self, version: Version) -> Option<String> {
        self.to_string_variables(version, &[])
    }
}

impl EventWithCommands for Video {
    fn commands(&self) -> &[Command] {
        &self.commands
    }

    fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    fn to_string_cmd(&self, version: Version) -> Option<String> {
        Some(format!(
            "{},{},{}{}",
            if self.short_hand {
                VIDEO_HEADER
            } else {
                VIDEO_HEADER_LONG
            },
            time_to_string(self.start_time, version),
            self.file_name.to_string(version).unwrap(),
            position_str(&self.position)
        ))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Break {
    pub start_time: Integer,
    pub end_time: Integer,
    short_hand: bool,
}

impl Break {
    pub fn new(start_time: Integer, end_time: Integer) -> Self {
        Self {
            start_time,
            end_time,
            short_hand: true,
        }
    }
}

pub const BREAK_HEADER: &str = "2";
pub const BREAK_HEADER_LONG: &str = "Break";

impl VersionedFromStr for Break {
    type Err = ParseBreakError;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let (_, (short_hand, start_time, end_time)) = tuple((
            alt((
                tag(BREAK_HEADER).map(|_| true),
                context(
                    ParseBreakError::WrongEventType.into(),
                    tag(BREAK_HEADER_LONG).map(|_| false),
                ),
            )),
            preceded(
                context(ParseBreakError::MissingStartTime.into(), comma()),
                start_time_offset(ParseBreakError::InvalidStartTime.into(), version),
            ),
            preceded(
                context(ParseBreakError::MissingEndTime.into(), comma()),
                end_time(ParseBreakError::InvalidEndTime.into(), version),
            ),
        ))(s)?;

        Ok(Some(Break {
            start_time,
            end_time,
            short_hand,
        }))
    }
}

impl VersionedToString for Break {
    fn to_string(&self, version: Version) -> Option<String> {
        Some(format!(
            "{},{},{}",
            if self.short_hand {
                BREAK_HEADER
            } else {
                BREAK_HEADER_LONG
            },
            time_to_string(self.start_time, version),
            time_to_string(self.end_time, version)
        ))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ColourTransformation {
    pub start_time: Integer,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

pub const COLOUR_TRANSFORMATION_HEADER: &str = "3";

impl VersionedFromStr for ColourTransformation {
    type Err = ParseColourTransformationError;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let (_, (start_time, red, green, blue)) = tuple((
            preceded(
                tuple((
                    context(
                        ParseColourTransformationError::WrongEventType.into(),
                        tag(COLOUR_TRANSFORMATION_HEADER),
                    ),
                    context(
                        ParseColourTransformationError::MissingStartTime.into(),
                        comma(),
                    ),
                )),
                start_time_offset(
                    ParseColourTransformationError::InvalidStartTime.into(),
                    version,
                ),
            ),
            preceded(
                context(ParseColourTransformationError::MissingRed.into(), comma()),
                context(
                    ParseColourTransformationError::InvalidRed.into(),
                    comma_field_type(),
                ),
            ),
            preceded(
                context(ParseColourTransformationError::MissingGreen.into(), comma()),
                context(
                    ParseColourTransformationError::InvalidGreen.into(),
                    comma_field_type(),
                ),
            ),
            preceded(
                context(ParseColourTransformationError::MissingBlue.into(), comma()),
                context(
                    ParseColourTransformationError::InvalidBlue.into(),
                    consume_rest_type(),
                ),
            ),
        ))(s)?;

        Ok(Some(ColourTransformation {
            start_time,
            red,
            green,
            blue,
        }))
    }
}

impl VersionedToString for ColourTransformation {
    fn to_string(&self, version: Version) -> Option<String> {
        if version < 14 {
            Some(format!(
                "{COLOUR_TRANSFORMATION_HEADER},{},{},{},{}",
                time_to_string(self.start_time, version),
                self.red,
                self.green,
                self.blue
            ))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
/// Legacy version of Sprite event.
pub struct SpriteLegacy {
    pub layer: LayerLegacy,
    pub origin: OriginTypeLegacy,
    pub file_name: FilePath,
    pub position: Option<Position>,
    pub commands: Vec<Command>,
}

pub const SPRITE_LEGACY_HEADER: &str = "4";

impl VersionedFromStr for SpriteLegacy {
    type Err = ParseSpriteLegacyError;

    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let (_, (layer, origin, (file_name, position))) = preceded(
            tuple((
                context(
                    ParseSpriteLegacyError::WrongEventType.into(),
                    tag(SPRITE_LEGACY_HEADER),
                ),
                context(ParseSpriteLegacyError::MissingLayer.into(), comma()),
            )),
            tuple((
                context(
                    ParseSpriteLegacyError::InvalidLayer.into(),
                    comma_field_versioned_type(version),
                ),
                preceded(
                    context(ParseSpriteLegacyError::MissingOrigin.into(), comma()),
                    context(
                        ParseSpriteLegacyError::InvalidOrigin.into(),
                        comma_field_versioned_type(version),
                    ),
                ),
                preceded(
                    context(ParseSpriteLegacyError::MissingFileName.into(), comma()),
                    file_name_and_position(
                        ParseSpriteLegacyError::MissingX.into(),
                        ParseSpriteLegacyError::InvalidX.into(),
                        ParseSpriteLegacyError::MissingY.into(),
                        ParseSpriteLegacyError::InvalidY.into(),
                    ),
                ),
            )),
        )(s)?;

        Ok(Some(SpriteLegacy {
            layer,
            origin,
            file_name,
            position,
            commands: Vec::new(),
        }))
    }
}

impl VersionedToString for SpriteLegacy {
    fn to_string(&self, version: Version) -> Option<String> {
        Some(format!(
            "{SPRITE_LEGACY_HEADER},{},{},{}{}",
            self.layer.to_string(version).unwrap(),
            self.origin.to_string(version).unwrap(),
            self.file_name.to_string(version).unwrap(),
            position_str(&self.position),
        ))
    }
}

impl EventWithCommands for SpriteLegacy {
    fn commands(&self) -> &[Command] {
        &self.commands
    }

    fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    fn to_string_cmd(&self, version: Version) -> Option<String> {
        self.to_string(version)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct AnimationLegacy {
    pub layer: LayerLegacy,
    pub origin: OriginTypeLegacy,
    pub file_name: FilePath,
    pub position: Option<Position>,
    pub commands: Vec<Command>,
}

pub const ANIMATION_LEGACY_HEADER: &str = "5";

impl VersionedFromStr for AnimationLegacy {
    type Err = ParseAnimationLegacyError;
    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let (_, (layer, origin, (file_name, position))) = preceded(
            tuple((
                context(
                    ParseAnimationLegacyError::WrongEventType.into(),
                    tag(ANIMATION_LEGACY_HEADER),
                ),
                context(ParseAnimationLegacyError::MissingLayer.into(), comma()),
            )),
            tuple((
                context(
                    ParseAnimationLegacyError::InvalidLayer.into(),
                    comma_field_versioned_type(version),
                ),
                preceded(
                    context(ParseAnimationLegacyError::MissingOrigin.into(), comma()),
                    context(
                        ParseAnimationLegacyError::InvalidOrigin.into(),
                        comma_field_versioned_type(version),
                    ),
                ),
                preceded(
                    context(ParseAnimationLegacyError::MissingFileName.into(), comma()),
                    file_name_and_position(
                        ParseAnimationLegacyError::MissingX.into(),
                        ParseAnimationLegacyError::InvalidX.into(),
                        ParseAnimationLegacyError::MissingY.into(),
                        ParseAnimationLegacyError::InvalidY.into(),
                    ),
                ),
            )),
        )(s)?;

        Ok(Some(AnimationLegacy {
            layer,
            origin,
            file_name,
            position,
            commands: Vec::new(),
        }))
    }
}

impl VersionedToString for AnimationLegacy {
    fn to_string(&self, version: Version) -> Option<String> {
        Some(format!(
            "{ANIMATION_LEGACY_HEADER},{},{},{}{}",
            self.layer.to_string(version).unwrap(),
            self.origin.to_string(version).unwrap(),
            self.file_name.to_string(version).unwrap(),
            position_str(&self.position),
        ))
    }
}

impl EventWithCommands for AnimationLegacy {
    fn commands(&self) -> &[Command] {
        &self.commands
    }
    fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }
    fn to_string_cmd(&self, version: Version) -> Option<String> {
        self.to_string(version)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SampleLegacy {
    pub time: Decimal,
    pub layer: LayerLegacy,
    pub file_name: FilePath,
    pub volume: Option<Volume>,
    pub commands: Vec<Command>,
}

pub const SAMPLE_LEGACY_HEADER: &str = "6";

impl VersionedFromStr for SampleLegacy {
    type Err = ParseSampleLegacyError;
    fn from_str(s: &str, version: Version) -> std::result::Result<Option<Self>, Self::Err> {
        let (_, (time, layer, file_name, volume)) = preceded(
            tuple((
                context(
                    ParseSampleLegacyError::WrongEventType.into(),
                    tag(SAMPLE_LEGACY_HEADER),
                ),
                context(ParseSampleLegacyError::MissingTime.into(), comma()),
            )),
            tuple((
                context(
                    ParseSampleLegacyError::InvalidTime.into(),
                    comma_field_type(),
                ),
                preceded(
                    context(ParseSampleLegacyError::MissingLayer.into(), comma()),
                    context(
                        ParseSampleLegacyError::InvalidLayer.into(),
                        comma_field_versioned_type(version),
                    ),
                ),
                preceded(
                    context(ParseSampleLegacyError::MissingFileName.into(), comma()),
                    comma_field().map(|f| f.into()),
                ),
                alt((
                    eof.map(|_| None),
                    preceded(
                        context(ParseSampleLegacyError::MissingVolume.into(), comma()),
                        context(
                            ParseSampleLegacyError::InvalidVolume.into(),
                            consume_rest_versioned_type(version),
                        ),
                    )
                    .map(Some),
                )),
            )),
        )(s)?;

        Ok(Some(SampleLegacy {
            time,
            layer,
            file_name,
            volume,
            commands: Vec::new(),
        }))
    }
}

impl VersionedToString for SampleLegacy {
    fn to_string(&self, version: Version) -> Option<String> {
        Some(format!(
            "{SAMPLE_LEGACY_HEADER},{},{},{}{}",
            self.time,
            self.layer.to_string(version).unwrap(),
            self.file_name.to_string(version).unwrap(),
            self.volume
                .map(|v| format!(",{}", v.to_string(version).unwrap()))
                .unwrap_or_default(),
        ))
    }
}

impl EventWithCommands for SampleLegacy {
    fn commands(&self) -> &[Command] {
        &self.commands
    }

    fn commands_mut(&mut self) -> &mut Vec<Command> {
        &mut self.commands
    }

    fn to_string_cmd(&self, version: Version) -> Option<String> {
        self.to_string(version)
    }
}
