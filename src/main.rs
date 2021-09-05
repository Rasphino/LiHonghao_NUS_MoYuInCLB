use std::collections::HashMap;
use std::io::{self, Read};
use std::str::FromStr;

use chrono::{Duration, NaiveDateTime};
use serde_json::json;

use robot_rate_calculator::{RobotStatus, RobotWorkTime};
use robot_rate_calculator::schema::RobotWorkSchema;

fn main() -> anyhow::Result<()> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let work_schema = serde_json::from_str::<RobotWorkSchema>(&buffer)?;

    let start_time = work_schema.shift.start.clone();
    let end_time = work_schema.shift.end.clone();
    let time_segments = {
        let mut v = vec![];
        v.push((work_schema.robo_rate.standard_day.start.clone(), work_schema.robo_rate.standard_day.end.clone()));
        v.push((work_schema.robo_rate.standard_night.start.clone(), work_schema.robo_rate.standard_night.end.clone()));
        v
    };
    let time_segment_to_value = {
        let mut m = HashMap::new();
        m.insert(RobotStatus::StandardDay, work_schema.robo_rate.standard_day.value);
        m.insert(RobotStatus::StandardNight, work_schema.robo_rate.standard_night.value);
        m.insert(RobotStatus::ExtraDay, work_schema.robo_rate.extra_day.value);
        m.insert(RobotStatus::ExtraNight, work_schema.robo_rate.extra_night.value);
        m
    };

    let t = RobotWorkTime::new(start_time, end_time, time_segments);
    let s = t.clone().into_iter().zip(std::iter::once((NaiveDateTime::from_str("2020-09-19T00:00:00").unwrap(), RobotStatus::Finish)).chain(t.into_iter())).skip(1)
        .fold(vec![Duration::zero(); 10], |mut acc, ((e, _), (s, status))| {
            acc[status as usize] = acc[status as usize] + (e - s);
            acc
        });
    let mut res = 0;
    for (k, v) in time_segment_to_value {
        res += s[k as usize].num_minutes() as u64 * v;
    }
    println!("{}", json!({ "value": res }).to_string());
    Ok(())
}
