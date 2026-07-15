use serde::{Deserialize, Serialize};

/// A note expression kind, mirroring CLAP's `clap_note_expression` ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpressionKind {
    Volume,
    Pan,
    Tuning,
    Vibrato,
    Expression,
    Brightness,
    Pressure,
}

impl ExpressionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ExpressionKind::Volume => "volume",
            ExpressionKind::Pan => "pan",
            ExpressionKind::Tuning => "tuning",
            ExpressionKind::Vibrato => "vibrato",
            ExpressionKind::Expression => "expression",
            ExpressionKind::Brightness => "brightness",
            ExpressionKind::Pressure => "pressure",
        }
    }
}

/// A single event crossing the plugin boundary, in either direction.
///
/// Field conventions follow CLAP: `channel` and `key` use `-1` as a wildcard,
/// `note_id` is a host- or plugin-assigned id with `-1` meaning unspecified,
/// and `velocity`/expression values are normalized doubles.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    NoteOn {
        channel: i16,
        key: i16,
        note_id: i32,
        velocity: f64,
    },
    NoteOff {
        channel: i16,
        key: i16,
        note_id: i32,
    },
    /// CLAP `note_end`: sent plugin→host when a voice has fully finished.
    NoteEnd {
        channel: i16,
        key: i16,
        note_id: i32,
    },
    NoteExpression {
        note_id: i32,
        kind: ExpressionKind,
        value: f64,
    },
    ParamValue {
        param_id: u32,
        value: f64,
    },
}

impl Event {
    pub fn note_on(channel: i16, key: i16, note_id: i32, velocity: f64) -> Self {
        Event::NoteOn {
            channel,
            key,
            note_id,
            velocity,
        }
    }

    pub fn note_off(channel: i16, key: i16, note_id: i32) -> Self {
        Event::NoteOff {
            channel,
            key,
            note_id,
        }
    }

    pub fn expression(note_id: i32, kind: ExpressionKind, value: f64) -> Self {
        Event::NoteExpression {
            note_id,
            kind,
            value,
        }
    }

    /// Shorthand for the expression kind microtonal plugins care about most.
    pub fn tuning(note_id: i32, semitones: f64) -> Self {
        Event::NoteExpression {
            note_id,
            kind: ExpressionKind::Tuning,
            value: semitones,
        }
    }

    pub fn param(param_id: u32, value: f64) -> Self {
        Event::ParamValue { param_id, value }
    }

    /// Attach a timestamp in seconds, producing a fixture-ready [`TimedEvent`].
    pub fn at_secs(self, at: f64) -> TimedEvent {
        TimedEvent { at, event: self }
    }

    pub fn is_note_expression(&self) -> bool {
        matches!(self, Event::NoteExpression { .. })
    }
}

/// An [`Event`] with a timestamp in seconds from the start of the render.
///
/// Fixtures use seconds rather than samples so the same fixture is meaningful
/// at any sample rate; the render engine converts to sample offsets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimedEvent {
    /// Seconds from the start of the render.
    pub at: f64,
    #[serde(flatten)]
    pub event: Event,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_reads_naturally() {
        let ev = Event::note_on(0, 60, 1, 0.8).at_secs(1.5);
        assert_eq!(ev.at, 1.5);
        assert_eq!(
            ev.event,
            Event::NoteOn {
                channel: 0,
                key: 60,
                note_id: 1,
                velocity: 0.8
            }
        );
    }

    #[test]
    fn json_shape_is_flat_and_tagged() {
        let json = serde_json::to_value(Event::tuning(1, 0.31).at_secs(0.1)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "at": 0.1,
                "type": "note_expression",
                "note_id": 1,
                "kind": "tuning",
                "value": 0.31,
            })
        );
    }
}
