use std::fmt;

use serde::{Deserialize, Serialize};

use crate::Event;

/// Which way an event crossed the plugin boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Host → plugin (`→` in the tap).
    ToPlugin,
    /// Plugin → host (`←` in the tap).
    FromPlugin,
}

impl Direction {
    pub fn arrow(self) -> char {
        match self {
            Direction::ToPlugin => '→',
            Direction::FromPlugin => '←',
        }
    }
}

/// One record of the event tap: an event that crossed the plugin boundary at
/// a given sample position.
///
/// The `Display` form is the tap's on-disk format — line-oriented with fixed
/// field order, so tap files diff cleanly and work as snapshot-test goldens:
///
/// ```text
///        480 → note_on    ch=0 key=60 id=1 vel=0.800
///       4800 ← note_expr  id=1 kind=tuning val=0.3100
///      96000 ← note_end   ch=0 key=60 id=1
/// ```
///
/// This format is append-only stable: new event kinds may add new mnemonics,
/// but existing lines never change shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TapEvent {
    /// Sample position from the start of processing.
    pub sample: u64,
    pub direction: Direction,
    pub event: Event,
}

impl fmt::Display for TapEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>10} {} ", self.sample, self.direction.arrow())?;
        match &self.event {
            Event::NoteOn {
                channel,
                key,
                note_id,
                velocity,
            } => {
                write!(
                    f,
                    "note_on    ch={channel} key={key} id={note_id} vel={velocity:.3}"
                )
            }
            Event::NoteOff {
                channel,
                key,
                note_id,
            } => {
                write!(f, "note_off   ch={channel} key={key} id={note_id}")
            }
            Event::NoteEnd {
                channel,
                key,
                note_id,
            } => {
                write!(f, "note_end   ch={channel} key={key} id={note_id}")
            }
            Event::NoteExpression {
                note_id,
                kind,
                value,
            } => {
                write!(
                    f,
                    "note_expr  id={note_id} kind={} val={value:.4}",
                    kind.as_str()
                )
            }
            Event::ParamValue { param_id, value } => {
                write!(f, "param      id={param_id} val={value:.4}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tap_lines_are_stable() {
        let cases = [
            (
                TapEvent {
                    sample: 480,
                    direction: Direction::ToPlugin,
                    event: Event::note_on(0, 60, 1, 0.8),
                },
                "       480 → note_on    ch=0 key=60 id=1 vel=0.800",
            ),
            (
                TapEvent {
                    sample: 4800,
                    direction: Direction::FromPlugin,
                    event: Event::tuning(1, 0.31),
                },
                "      4800 ← note_expr  id=1 kind=tuning val=0.3100",
            ),
            (
                TapEvent {
                    sample: 96000,
                    direction: Direction::FromPlugin,
                    event: Event::NoteEnd {
                        channel: 0,
                        key: 60,
                        note_id: 1,
                    },
                },
                "     96000 ← note_end   ch=0 key=60 id=1",
            ),
            (
                TapEvent {
                    sample: 0,
                    direction: Direction::ToPlugin,
                    event: Event::param(5, 0.5),
                },
                "         0 → param      id=5 val=0.5000",
            ),
        ];
        for (event, expected) in cases {
            assert_eq!(event.to_string(), expected);
        }
    }
}
