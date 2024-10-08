pub mod error;
pub mod types;

use std::fmt::Debug;
use std::num::{IntErrorKind, ParseIntError};
use std::path::PathBuf;

use rust_decimal::Decimal;
use crate::parsers::comma;
use nom::bytes::complete::take_till;
use nom::combinator::map_res;
use nom::multi::separated_list0;
use nom::Finish;
use rust_decimal_macros::dec;

use crate::helper;
use crate::helper::macros::*;

use crate::osu_file::Integer;

pub use error::*;
pub use types::*;

versioned_field!(AudioFilename, PathBuf, no_versions, |s| { Ok(PathBuf::from(s)) } -> (), |v| { v.display().to_string() }, PathBuf::from(""));
versioned_field!(AudioLeadIn, Integer, no_versions, |s| { s.parse() } -> ParseIntError,, 0);
versioned_field!(AudioHash, String, no_versions, |s| { Ok(s.to_string()) } -> (),
    |v, version| { if version > 13 { None } else { Some(v.to_string()) } },
    |version| { if version > 13 { None } else { Some(String::new())}
});
versioned_field!(PreviewTime, Integer, no_versions, |s| { s.parse() } -> ParseIntError,, -1);
versioned_field!(StackLeniency, Decimal, no_versions, |s| { s.parse() } -> rust_decimal::Error,, Decimal::from(dec!(0.7)));
versioned_field!(LetterboxInBreaks, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, false);
versioned_field!(StoryFireInFront, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, true);
versioned_field!(UseSkinSprites, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, false);
versioned_field!(AlwaysShowPlayfield, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, false);
versioned_field!(SkinPreference, String, no_versions, |s| { Ok(s.to_string()) } -> (),, String::new());
versioned_field!(EpilepsyWarning, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, false);
versioned_field!(CountdownOffset, Integer, no_versions, |s| { s.parse() } -> ParseIntError,, 0);
versioned_field!(SpecialStyle, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, false);
versioned_field!(WidescreenStoryboard, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, false);
versioned_field!(SamplesMatchPlaybackRate, bool, no_versions, |s| { helper::parse_zero_one_bool(s) } -> helper::ParseZeroOneBoolError, boolean, false);
versioned_field!(EditorBookmarks, Vec<Integer>, no_versions, |s| {
    let bookmark = map_res(take_till(|c| c == ','), |s: &str| s.parse::<Integer>());
    let mut bookmarks = separated_list0(comma::<nom::error::Error<_>>(), bookmark);
    let input_len = s.len();

    let (s, bookmarks) = bookmarks(s).finish().unwrap();

    if s.is_empty() {
        Ok(bookmarks)
    } else {
        let (_, s) = {
            let s = if s.len() < input_len {
                match s.strip_prefix(',') {
                    Some(s) => s,
                    None => s,
                }
            } else {
                s
            };

            take_till::<_, _, nom::error::Error<_>>(|c| c == ',')(s).unwrap()
        };

        // re-parse to get error
        let err = s.parse::<Integer>().unwrap_err();

        let err = if let IntErrorKind::Empty = err.kind() {
            ParseError::InvalidCommaList
        } else {
            ParseError::ParseIntError(err)
        };

        Err(err)
    }
} -> ParseError,
|v, version| {
    if version > 5 {
        return None;
    }

    Some(v.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","))
},
);
versioned_field!(
    EditorDistanceSpacing,
    Decimal,
    no_versions,
    |s| { s.parse() } -> rust_decimal::Error,
    |v, version| {
        if version > 5 {
            return None;
        }

        Some(v.to_string())
    },
);

general_section!(
    /// A struct representing the general section of an osu file.
    pub struct General {
        /// The name of the beatmap.
        pub audio_filename: AudioFilename,
        /// Milliseconds of silence before the audio starts playing.
        pub audio_lead_in: AudioLeadIn,
        /// Deprecated.
        pub audio_hash: AudioHash,
        /// Time in milliseconds when the audio preview should start.
        /// - Defaults to `-1`.
        pub preview_time: PreviewTime,
        /// Speed of the countdown before the first hit object.
        /// - Defaults to `Normal`.
        pub countdown: Countdown,
        /// Sample set that will be used if timing points do not override it.
        /// - Defaults to `Normal`.
        pub sample_set: SampleSet,
        /// Multiplier for the threshold in time where hit objects placed close together stack.
        /// - Defaults to `0.7`.
        pub stack_leniency: StackLeniency,
        /// Game mode.
        /// - Defaults to `osu`.
        pub mode: Mode,
        /// Whether or not breaks have a letterboxing effect.
        /// - Defaults to `false`.
        pub letterbox_in_breaks: LetterboxInBreaks,
        /// Deprecated.
        /// - Defaults to `true`.
        pub story_fire_in_front: StoryFireInFront,
        /// Whether or not the storyboard can use the user's skin images.
        /// - Defaults to `false`.
        pub use_skin_sprites: UseSkinSprites,
        /// Deprecated.
        /// - Defaults to `false`.
        pub always_show_playfield: AlwaysShowPlayfield,
        /// Draw order of hit circle overlays compared to hit numbers.
        /// - Defaults to `NoChange`.
        pub overlay_position: OverlayPosition,
        /// Preferred skin to use during gameplay.
        pub skin_preference: SkinPreference,
        /// Whether or not a warning about flashing colours should be shown at the beginning of the map.
        /// - Defaults to `false`.
        pub epilepsy_warning: EpilepsyWarning,
        /// Time in beats that the countdown starts before the first hit object.
        /// - Defaults to `0`.
        pub countdown_offset: CountdownOffset,
        /// Whether or not the "N+1" style key layout is used for osu!mania.
        /// - Defaults to `false`.
        pub special_style: SpecialStyle,
        /// Whether or not the storyboard allows widescreen viewing.
        /// - Defaults to `false`.
        pub widescreen_storyboard: WidescreenStoryboard,
        /// Whether or not sound samples will change rate when playing with speed-changing mods.
        /// - Defaults to `false`.
        pub samples_match_playback_rate: SamplesMatchPlaybackRate,
        /// Deprecated.
        pub editor_bookmarks: EditorBookmarks,
        /// Deprecated.
        pub editor_distance_spacing: EditorDistanceSpacing,
    },
    ParseError,
    " ",
);
