use serde::{Serialize, Deserialize};
use chrono::{NaiveDateTime, NaiveTime};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotWorkSchema {
    pub shift: Shift,
    pub robo_rate: RoboRate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shift {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoboRate {
    pub standard_day: StandardDay,
    pub standard_night: StandardNight,
    pub extra_day: ExtraDay,
    pub extra_night: ExtraNight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardDay {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardNight {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraDay {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraNight {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub value: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_test() {
        let json_input = r#"{
  "shift": {
      "start": "2038-01-01T20:15:00",
      "end": "2038-01-02T04:15:00"
  },
  "roboRate": {
    "standardDay": {
      "start": "07:00:00",
      "end": "23:00:00",
      "value": 20
    },
    "standardNight": {
      "start": "23:00:00",
      "end": "07:00:00",
      "value": 25
    },
    "extraDay": {
      "start": "07:00:00",
      "end": "23:00:00",
      "value": 30
    },
    "extraNight": {
      "start": "23:00:00",
      "end": "07:00:00",
      "value": 35
    }
  }
}"#;
        let s = serde_json::from_str::<RobotWorkSchema>(json_input).unwrap();
        dbg!(s);
    }
}
