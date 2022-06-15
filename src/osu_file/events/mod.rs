pub mod error;
pub mod storyboard;

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    combinator::{cut, eof, fail, rest},
    error::context,
    sequence::{preceded, terminated, tuple},
    Finish, Parser,
};

use crate::{osu_file::events::storyboard::sprites, parsers::*};

use self::storyboard::{cmds::CommandProperties, error::ObjectParseError, sprites::Object};

use super::{types::Error, Integer, Position, Version};

pub use self::error::*;

#[derive(Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Events(pub Vec<Event>);

impl Version for Events {
    type ParseError = Error<ParseError>;

    // TODO check versions
    fn from_str(s: &str, _: usize) -> std::result::Result<Option<Self>, Self::ParseError> {
        let mut events = Events(Vec::new());

        let mut comment = preceded::<_, _, _, nom::error::Error<_>, _, _>(tag("//"), rest);

        for (line_index, line) in s.lines().enumerate() {
            if !line.trim().is_empty() {
                if let Ok((_, comment)) = comment(line) {
                    events.0.push(Event::Comment(comment.to_string()));
                } else {
                    let indent = take_while::<_, _, nom::error::Error<_>>(|c| c == ' ' || c == '_');

                    let indent = indent(line).unwrap().1.len();

                    // its a storyboard command
                    if indent > 0 {
                        match events.0.last_mut() {
                            Some(Event::Storyboard(sprite)) => Error::new_from_result_into(
                                sprite.try_push_cmd(
                                    Error::new_from_result_into(line.parse(), line_index)?,
                                    indent,
                                ),
                                line_index,
                            )?,
                            _ => {
                                return Err(Error::new(
                                    ParseError::StoryboardCmdWithNoSprite,
                                    line_index,
                                ))
                            }
                        }
                    } else {
                        // is it a storyboard object?

                        match line.parse() {
                            Ok(object) => {
                                events.0.push(Event::Storyboard(object));
                            }
                            Err(err) => {
                                if let ObjectParseError::UnknownObjectType(_) = err {
                                    let (_, (_, _, start_time, _)) = tuple((
                                        comma_field(),
                                        context("missing_start_time", comma()),
                                        context("invalid_end_time", comma_field_type()),
                                        rest,
                                    ))(
                                        line
                                    )
                                    .finish()
                                    .map_err(|err: nom::error::VerboseError<_>| {
                                        for (_, err) in err.errors {
                                            if let nom::error::VerboseErrorKind::Context(context) =
                                                err
                                            {
                                                let err = match context {
                                                    "missing_start_time" => {
                                                        ParseError::MissingStartTime
                                                    }
                                                    "invalid_end_time" => {
                                                        ParseError::ParseStartTime
                                                    }
                                                    _ => unreachable!(),
                                                };

                                                return Error::new(err, line_index);
                                            }
                                        }

                                        unimplemented!("I somehow forgot to implement a context");
                                    })?;

                                    events.0.push(Event::NormalEvent {
                                        start_time,
                                        event_params: Error::new_from_result_into(
                                            line.parse(),
                                            line_index,
                                        )?,
                                    })
                                } else {
                                    return Err(Error::new(
                                        ParseError::StoryboardObjectParseError(err),
                                        line_index,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Some(events))
    }

    fn to_string(&self, _: usize) -> Option<String> {
        Some(
            self.0
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    fn default(_: usize) -> Option<Self> {
        Some(Events(Vec::new()))
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Event {
    Comment(String),
    NormalEvent {
        start_time: Integer,
        event_params: EventParams,
    },
    Storyboard(Object),
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let event_str = match self {
            Event::Comment(comment) => format!("//{comment}"),
            Event::NormalEvent {
                start_time,
                event_params,
            } => {
                let position_str = |position: &Option<Position>| match position {
                    Some(position) => format!(",{},{}", position.x, position.y),
                    None => String::new(),
                };

                match event_params {
                    EventParams::Background(background) => format!(
                        "0,{start_time},{}{}",
                        background.filename.to_string_lossy(),
                        position_str(&background.position),
                    ),
                    EventParams::Video(video) => format!(
                        "{},{start_time},{}{}",
                        if video.short_hand { "1" } else { "Video" },
                        video.filename.to_string_lossy(),
                        position_str(&video.position),
                    ),
                    EventParams::Break(_break) => {
                        format!(
                            "{},{start_time},{}",
                            if _break.short_hand { "2" } else { "Break" },
                            _break.end_time
                        )
                    }
                    EventParams::ColourTransformation(transformation) => format!(
                        "3,{start_time},{},{},{}",
                        transformation.red, transformation.green, transformation.blue
                    ),
                }
            }
            Event::Storyboard(object) => {
                let pos_str = format!("{},{}", object.position.x, object.position.y);

                let object_str = match &object.object_type {
                    sprites::ObjectType::Sprite(sprite) => format!(
                        "Sprite,{},{},{},{}",
                        object.layer,
                        object.origin,
                        // TODO make sure if the filepath has spaces, use the quotes no matter what, but don't add quotes if it already has
                        sprite.filepath.to_string_lossy(),
                        pos_str
                    ),
                    sprites::ObjectType::Animation(anim) => {
                        format!(
                            "Animation,{},{},{},{},{},{},{}",
                            object.layer,
                            object.origin,
                            anim.filepath.to_string_lossy(),
                            pos_str,
                            anim.frame_count,
                            anim.frame_delay,
                            anim.loop_type
                        )
                    }
                };

                let cmds = {
                    if object.commands.is_empty() {
                        None
                    } else {
                        let mut builder = Vec::new();
                        let mut indentation = 1usize;

                        for cmd in &object.commands {
                            builder.push(format!("{}{cmd}", " ".repeat(indentation)));

                            if let CommandProperties::Loop { commands, .. }
                            | CommandProperties::Trigger { commands, .. } = &cmd.properties
                            {
                                if commands.is_empty() {
                                    continue;
                                }

                                let starting_indentation = indentation;
                                indentation += 1;

                                let mut current_cmds = commands;
                                let mut current_index = 0;
                                // stack of commands, index, and indentation
                                let mut cmds_stack = Vec::new();

                                loop {
                                    let cmd = &current_cmds[current_index];
                                    current_index += 1;

                                    builder.push(format!("{}{cmd}", " ".repeat(indentation)));
                                    match &cmd.properties {
                                        CommandProperties::Loop { commands, .. }
                                        | CommandProperties::Trigger { commands, .. }
                                            if !commands.is_empty() =>
                                        {
                                            // save the current cmds and index
                                            // ignore if index is already at the end of the current cmds
                                            if current_index < current_cmds.len() {
                                                cmds_stack.push((
                                                    current_cmds,
                                                    current_index,
                                                    indentation,
                                                ));
                                            }

                                            current_cmds = commands;
                                            current_index = 0;
                                            indentation += 1;
                                        }
                                        _ => {
                                            if current_index >= current_cmds.len() {
                                                // check for end of commands
                                                match cmds_stack.pop() {
                                                    Some((
                                                        last_cmds,
                                                        last_index,
                                                        last_indentation,
                                                    )) => {
                                                        current_cmds = last_cmds;
                                                        current_index = last_index;
                                                        indentation = last_indentation;
                                                    }
                                                    None => break,
                                                }
                                            }
                                        }
                                    }
                                }

                                indentation = starting_indentation;
                            }
                        }

                        Some(builder.join("\n"))
                    }
                };

                match cmds {
                    Some(cmds) => format!("{object_str}\n{cmds}"),
                    None => object_str,
                }
            }
        };

        write!(f, "{event_str}")
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Background {
    pub filename: PathBuf,
    pub position: Option<Position>,
}

impl Background {
    pub fn new(filename: &Path, position: Option<Position>) -> Self {
        Self {
            filename: filename.to_path_buf(),
            position,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Video {
    pub filename: PathBuf,
    pub position: Option<Position>,
    short_hand: bool,
}

impl Video {
    pub fn new(filename: PathBuf, position: Option<Position>) -> Self {
        Self {
            filename,
            position,
            short_hand: true,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Break {
    pub end_time: Integer,
    short_hand: bool,
}

impl Break {
    pub fn new(end_time: Integer) -> Self {
        Self {
            end_time,
            short_hand: true,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ColourTransformation {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum EventParams {
    Background(Background),
    Video(Video),
    Break(Break),
    ColourTransformation(ColourTransformation),
}

impl FromStr for EventParams {
    type Err = EventParamsParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let start_time = comma_field;
        let file_name = || comma_field().map(PathBuf::from);
        let coordinates = || {
            alt((
                eof.map(|_| None),
                tuple((
                    preceded(
                        context(EventParamsParseError::MissingXOffset.into(), comma()),
                        context(
                            EventParamsParseError::InvalidXOffset.into(),
                            comma_field_type(),
                        ),
                    ),
                    preceded(
                        context(EventParamsParseError::MissingYOffset.into(), comma()),
                        context(
                            EventParamsParseError::InvalidYOffset.into(),
                            consume_rest_type(),
                        ),
                    ),
                ))
                .map(|(x, y)| Some((x, y))),
            ))
            .map(|position| position.map(|(x, y)| Position { x, y }))
        };
        let file_name_and_coordinates = || tuple((file_name(), coordinates()));
        let end_time = consume_rest_type();

        let background = preceded(
            tuple((
                tag("0"),
                cut(tuple((
                    context(EventParamsParseError::MissingStartTime.into(), comma()),
                    start_time(),
                    context(EventParamsParseError::MissingFileName.into(), comma()),
                ))),
            )),
            cut(file_name_and_coordinates()),
        )
        .map(|(filename, position)| EventParams::Background(Background { filename, position }));
        let video = tuple((
            terminated(
                alt((tag("1").map(|_| true), tag("Video").map(|_| false))),
                cut(tuple((
                    context(EventParamsParseError::MissingStartTime.into(), comma()),
                    start_time(),
                    context(EventParamsParseError::MissingFileName.into(), comma()),
                ))),
            ),
            cut(file_name_and_coordinates()),
        ))
        .map(|(short_hand, (filename, position))| {
            EventParams::Video(Video {
                filename,
                position,
                short_hand,
            })
        });
        let break_ = tuple((
            terminated(
                alt((tag("2").map(|_| true), tag("Break").map(|_| false))),
                cut(tuple((
                    context(EventParamsParseError::MissingStartTime.into(), comma()),
                    start_time(),
                    context(EventParamsParseError::MissingEndTime.into(), comma()),
                ))),
            ),
            cut(context(
                EventParamsParseError::InvalidEndTime.into(),
                end_time,
            )),
        ))
        .map(|(short_hand, end_time)| {
            EventParams::Break(Break {
                end_time,
                short_hand,
            })
        });
        let colour_transformation = preceded(
            tuple((
                tag("3"),
                cut(tuple((
                    context(EventParamsParseError::MissingStartTime.into(), comma()),
                    start_time(),
                    context(EventParamsParseError::MissingRed.into(), comma()),
                ))),
            )),
            cut(tuple((
                context(EventParamsParseError::InvalidRed.into(), comma_field_type()),
                preceded(
                    context(EventParamsParseError::MissingGreen.into(), comma()),
                    context(
                        EventParamsParseError::InvalidGreen.into(),
                        comma_field_type(),
                    ),
                ),
                preceded(
                    context(EventParamsParseError::MissingBlue.into(), comma()),
                    context(
                        EventParamsParseError::InvalidBlue.into(),
                        consume_rest_type(),
                    ),
                ),
            ))),
        )
        .map(|(red, green, blue)| {
            EventParams::ColourTransformation(ColourTransformation { red, green, blue })
        });

        let result = alt((
            background,
            video,
            break_,
            colour_transformation,
            context(EventParamsParseError::UnknownEventType.into(), fail),
        ))(s)?;

        Ok(result.1)
    }
}
