use serde::{Deserialize, Serialize};

use crate::TimedEvent;

/// A serializable sequence of timed input events.
///
/// A fixture is the file form of the builder API: `plause render --events`
/// consumes fixtures as JSON, and Rust test code can construct the identical
/// sequence programmatically. Events need not be pre-sorted; the engine orders
/// them by timestamp when scheduling.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    pub events: Vec<TimedEvent>,
}

impl Fixture {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an event, builder-style.
    pub fn event(mut self, event: TimedEvent) -> Self {
        self.events.push(event);
        self
    }

    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Fixture serialization cannot fail")
    }
}

impl FromIterator<TimedEvent> for Fixture {
    fn from_iter<I: IntoIterator<Item = TimedEvent>>(iter: I) -> Self {
        Fixture {
            events: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Event;

    fn example() -> Fixture {
        Fixture::new()
            .event(Event::note_on(0, 60, 1, 0.8).at_secs(0.0))
            .event(Event::tuning(1, 0.31).at_secs(0.1))
            .event(Event::note_off(0, 60, 1).at_secs(2.0))
    }

    #[test]
    fn json_round_trip() {
        let fixture = example();
        assert_eq!(Fixture::from_json(&fixture.to_json()).unwrap(), fixture);
    }

    #[test]
    fn hand_written_json_parses() {
        let fixture = Fixture::from_json(
            r#"{
                "events": [
                    { "at": 0.0, "type": "note_on", "channel": 0, "key": 60, "note_id": 1, "velocity": 0.8 },
                    { "at": 0.1, "type": "note_expression", "note_id": 1, "kind": "tuning", "value": 0.31 },
                    { "at": 2.0, "type": "note_off", "channel": 0, "key": 60, "note_id": 1 }
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(fixture, example());
    }
}
