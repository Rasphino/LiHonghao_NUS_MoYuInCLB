use std::io::{self, Read};
use std::str::FromStr;

use chrono::{Duration, NaiveDateTime, Weekday};
use serde_json::json;

use robot_rate_calculator::{RobotWorkTime, TimeRange};
use robot_rate_calculator::schema::RobotWorkSchema;

fn weekend() -> impl Iterator<Item=Weekday> {
    use Weekday::*;
    vec![Sat, Sun].into_iter()
}

fn weekday() -> impl Iterator<Item=Weekday> {
    use Weekday::*;
    vec![Mon, Tue, Wed, Thu, Fri].into_iter()
}

fn main() -> anyhow::Result<()> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let work_schema = serde_json::from_str::<RobotWorkSchema>(&buffer)?;

    let start_time = work_schema.shift.start;
    let end_time = work_schema.shift.end;
    let time_ranges = vec![
        TimeRange::new((work_schema.robo_rate.standard_day.start, work_schema.robo_rate.standard_day.end), weekday()),
        TimeRange::new((work_schema.robo_rate.standard_night.start, work_schema.robo_rate.standard_night.end), weekday()),
        TimeRange::new((work_schema.robo_rate.extra_day.start, work_schema.robo_rate.extra_day.end), weekend()),
        TimeRange::new((work_schema.robo_rate.extra_night.start, work_schema.robo_rate.extra_night.end), weekend()),
    ];
    let time_range_to_value = vec![
        work_schema.robo_rate.standard_day.value,
        work_schema.robo_rate.standard_night.value,
        work_schema.robo_rate.extra_day.value,
        work_schema.robo_rate.extra_night.value,
    ];
    let n = time_ranges.len();

    let t = RobotWorkTime::new(start_time, end_time, time_ranges);
    let s = t.clone().into_iter().zip(std::iter::once((NaiveDateTime::from_str("2020-09-19T00:00:00").unwrap(), None)).chain(t.into_iter())).skip(1)
        .fold(vec![Duration::zero(); n], |mut acc, ((e, _), (s, status))| {
            if let Some(idx) = status {
                acc[idx] = acc[idx] + (e - s);
            }
            acc
        });

    let res: u64 = s.iter().zip(time_range_to_value.iter()).map(|(duration, rate)| duration.num_minutes() as u64 * *rate).sum();

    println!("{}", json!({ "value": res }).to_string());
    Ok(())
}
