pub mod error;
pub mod storyboard;

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use nom::{
    bytes::complete::{tag, take_while},
    combinator::rest,
    error::context,
    sequence::{preceded, tuple},
    Finish,
};

use crate::{
    osu_file::events::storyboard::sprites,
    parsers::{comma, comma_field, comma_field_i32},
};

use self::storyboard::{
    cmds::{Command, CommandProperties},
    error::ObjectParseError,
    sprites::Object,
};

use super::{types::Error, Integer, Position};

pub use self::error::*;

#[derive(Default, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Events(pub Vec<Event>);

impl Display for Events {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

impl FromStr for Events {
    type Err = crate::osu_file::types::Error<ParseError>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut events = Events::default();

        let comment = preceded::<_, _, _, nom::error::Error<_>, _, _>(tag("//"), rest);

        let mut line_index = 0;

        for line in s.lines() {
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
                                    let missing_start_time = "missing_start_time";
                                    let invalid_end_time = "invalid_end_time";

                                    let (_, (_, _, start_time, _)) = tuple((
                                        comma_field(),
                                        context(missing_start_time, comma()),
                                        context(invalid_end_time, comma_field_i32()),
                                        take_while(|_| true),
                                    ))(
                                        line
                                    )
                                    .finish()
                                    .map_err(|err: nom::error::VerboseError<_>| {
                                        for (input, err) in err.errors {
                                            if let nom::error::VerboseErrorKind::Context(context) =
                                                err
                                            {
                                                let err = match context {
                                                    missing_start_time => {
                                                        ParseError::MissingStartTime
                                                    }
                                                    invalid_end_time => ParseError::ParseStartTime,
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

            line_index += 1;
        }

        Ok(events)
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
            // TODO do normal event type's value storage such as if being 0, it will remember that for 1:1 matching
            Event::NormalEvent {
                start_time,
                event_params,
            } => {
                let position_str = |position: &Position| {
                    if position.x == 0 && position.y == 0 {
                        String::new()
                    } else {
                        format!(",{},{}", position.x, position.y)
                    }
                };

                match event_params {
                    // background by default doesn't print the shorthand for some reason
                    EventParams::Background(background) => format!(
                        "0,{start_time},{},{},{}",
                        background.filename.to_string_lossy(),
                        background.position.x,
                        background.position.y,
                    ),
                    EventParams::Video(video) => format!(
                        "1,{start_time},{}{}",
                        video.filename.to_string_lossy(),
                        position_str(&video.position),
                    ),
                    EventParams::Break(_break) => format!("2,{start_time},{}", _break.end_time),
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
                        Some(command_recursive_display(&object.commands, 1).join("\n"))
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

fn command_recursive_display(commands: &[Command], indentation: usize) -> Vec<String> {
    let mut cmd_lines = Vec::with_capacity(commands.len());

    for cmd in commands {
        cmd_lines.push(format!("{}{cmd}", " ".repeat(indentation)));

        if let CommandProperties::Loop { commands, .. }
        | CommandProperties::Trigger { commands, .. } = &cmd.properties
        {
            for command_str in command_recursive_display(commands, indentation + 1) {
                cmd_lines.push(command_str);
            }
        }
    }

    cmd_lines
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Background {
    pub filename: PathBuf,
    pub position: Position,
}

impl Background {
    pub fn new(filename: &Path, position: Position) -> Self {
        Self {
            filename: filename.to_path_buf(),
            position,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Video {
    pub filename: PathBuf,
    pub position: Position,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Break {
    pub end_time: Integer,
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
        todo!()
    }
}

impl<'a, T> TryFrom<(&str, &mut T)> for EventParams
where
    T: Iterator<Item = &'a str> + Clone + std::fmt::Debug,
{
    type Error = EventParamsParseError;

    fn try_from((event_type, event_params): (&str, &mut T)) -> Result<Self, Self::Error> {
        let event_type = event_type.trim();

        match event_type {
            "0" | "1" | "Video" => {
                let filename = event_params
                    .next()
                    .ok_or(EventParamsParseError::MissingField("filename"))?;
                let position = {
                    let mut has_position = false;

                    let x = match event_params.next() {
                        Some(x) => {
                            has_position = true;
                            x
                        }
                        None => "0",
                    };
                    let x = x
                        .parse()
                        .map_err(|err| EventParamsParseError::ParseFieldError {
                            source: Box::new(err),
                            value: x.to_string(),
                            field_name: "xOffset",
                        })?;

                    let y = match event_params.next() {
                        Some(y) => {
                            if has_position {
                                y
                            } else {
                                return Err(EventParamsParseError::MissingField("xOffset"));
                            }
                        }
                        None => {
                            if has_position {
                                return Err(EventParamsParseError::MissingField("yOffset"));
                            } else {
                                "0"
                            }
                        }
                    };
                    let y = y
                        .parse()
                        .map_err(|err| EventParamsParseError::ParseFieldError {
                            source: Box::new(err),
                            value: y.to_string(),
                            field_name: "yOffset",
                        })?;

                    Position { x, y }
                };

                let filename = Path::new(filename);

                if event_type == "0" {
                    Ok(EventParams::Background(Background {
                        filename: filename.to_path_buf(),
                        position,
                    }))
                } else {
                    Ok(EventParams::Video(Video {
                        filename: filename.to_path_buf(),
                        position,
                    }))
                }
            }
            "2" | "Break" => {
                let end_time = event_params
                    .next()
                    .ok_or(EventParamsParseError::MissingField("endTime"))?;
                let end_time =
                    end_time
                        .parse()
                        .map_err(|err| EventParamsParseError::ParseFieldError {
                            source: Box::new(err),
                            value: end_time.to_string(),
                            field_name: "endTime",
                        })?;

                Ok(EventParams::Break(Break { end_time }))
            }
            "3" => {
                let red = event_params
                    .next()
                    .ok_or(EventParamsParseError::MissingField("red"))?;
                let red = red
                    .parse()
                    .map_err(|err| EventParamsParseError::ParseFieldError {
                        source: Box::new(err),
                        value: red.to_string(),
                        field_name: "red",
                    })?;

                let green = event_params
                    .next()
                    .ok_or(EventParamsParseError::MissingField("green"))?;
                let green =
                    green
                        .parse()
                        .map_err(|err| EventParamsParseError::ParseFieldError {
                            source: Box::new(err),
                            value: green.to_string(),
                            field_name: "green",
                        })?;

                let blue = event_params
                    .next()
                    .ok_or(EventParamsParseError::MissingField("blue"))?;
                let blue = blue
                    .parse()
                    .map_err(|err| EventParamsParseError::ParseFieldError {
                        source: Box::new(err),
                        value: blue.to_string(),
                        field_name: "blue",
                    })?;

                Ok(EventParams::ColourTransformation(ColourTransformation {
                    red,
                    green,
                    blue,
                }))
            }
            _ => Err(EventParamsParseError::UnknownParamType(
                event_type.to_string(),
            )),
        }
    }
}
