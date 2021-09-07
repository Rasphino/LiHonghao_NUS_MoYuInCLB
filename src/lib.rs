use std::collections::HashSet;

use chrono::{Datelike, Duration, NaiveDateTime, NaiveTime, Weekday};

pub mod schema;

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct TimeRange {
    start: NaiveTime,
    end: NaiveTime,
    valid_weekdays: HashSet<Weekday>,
}

impl TimeRange {
    pub fn new(range: (NaiveTime, NaiveTime), valid_weekdays: impl Iterator<Item=Weekday>) -> Self {
        Self {
            start: range.0,
            end: range.1,
            valid_weekdays: valid_weekdays.into_iter().collect::<HashSet<_>>(),
        }
    }

    pub fn contains(&self, datetime: NaiveDateTime) -> bool {
        if self.valid_weekdays.contains(&datetime.date().weekday()) {
            let t = datetime.time();
            if (self.start < self.end && t >= self.start && t < self.end) || (self.start > self.end && (t >= self.start || t < self.end)) {
                return true;
            }
        }
        false
    }

    pub fn get_next_range_start_at(&self, datetime: NaiveDateTime) -> Option<(NaiveDateTime, NaiveDateTime)> {
        let t = datetime.time();
        let d = datetime.date();

        let ans = if self.start < self.end {
            if t >= self.start && t < self.end {
                Some((datetime, d.and_time(self.end)))
            } else {
                None
            }
        } else {
            if t < self.start && t > self.end {
                None
            } else if t >= self.start {
                // from datetime to the end of the day
                Some((datetime, d.and_hms(0, 0, 0) + Duration::days(1)))
            } else if t < self.end {
                // from datetime to the end of this range
                Some((datetime, d.and_time(self.end)))
            } else {
                None
            }
        };

        ans.map(|(mut s, mut e)| {
            while !self.valid_weekdays.contains(&s.date().weekday()) {
                s += Duration::days(1);
                e += Duration::days(1);
            }
            (s, e)
        })
    }
}

/// `BreakIterator` produces a infinite sequence of time points at which the robot need to have a break.
#[derive(Eq, PartialEq, Debug, Clone)]
struct BreakIterator {
    start: NaiveDateTime,
    work_duration: Duration,
    rest_duration: Duration,
}

impl Iterator for BreakIterator {
    type Item = (NaiveDateTime, NaiveDateTime);

    fn next(&mut self) -> Option<Self::Item> {
        let work_end = self.start + self.work_duration;
        let r = (work_end, work_end + self.rest_duration);
        self.start = work_end + self.rest_duration;
        Some(r)
    }
}

#[derive(Clone)]
pub struct RobotWorkTime {
    start: NaiveDateTime,
    end: NaiveDateTime,
    time_range: Vec<TimeRange>,
}

impl RobotWorkTime {
    pub fn new(start: NaiveDateTime, end: NaiveDateTime, time_range: Vec<TimeRange>) -> Self {
        Self { start, end, time_range }
    }

    pub fn into_iter(self) -> RobotWorkTimeIterator {
        let RobotWorkTime { time_range, start, end: _ } = self;

        let mut time_ranges_iter = TimeRangesIterator::new(start, time_range).unwrap();
        let cur = time_ranges_iter.next().unwrap();
        let break_iter = BreakIterator {
            start: self.start,
            work_duration: Duration::hours(8),
            rest_duration: Duration::hours(1),
        };

        RobotWorkTimeIterator {
            cur: (cur.0, Some(cur.1)),
            end: self.end,
            time_ranges_iter,
            break_iter,
            breaking: None,
            is_finish: false,
        }
    }
}

/// `TimeSegmentsIterator` produces a infinite sequence of time points, at which the robot status (and the corresponding rates) changed.
#[derive(Eq, PartialEq, Debug, Clone)]
struct TimeRangesIterator {
    cur: (NaiveDateTime, usize),
    time_ranges: Vec<TimeRange>,
}

impl TimeRangesIterator {
    pub fn new(start: NaiveDateTime, time_ranges: Vec<TimeRange>) -> Option<Self> {
        let mut next_idx = None;
        for (idx, range) in time_ranges.iter().enumerate() {
            if range.contains(start) {
                next_idx = Some(idx)
            }
        }
        Some(Self {
            cur: (start, next_idx?),
            time_ranges,
        })
    }
}

impl Iterator for TimeRangesIterator {
    type Item = (NaiveDateTime, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.cur;
        let (date_time, _idx) = self.cur;

        let (_, next_dt) = self.time_ranges.iter()
            .filter_map(|time_range| {
                time_range.get_next_range_start_at(date_time)
            })
            .min_by_key(|(_, next_dt)| *next_dt)
            .unwrap();

        let mut next_idx = None;
        for (idx, range) in self.time_ranges.iter().enumerate() {
            if range.contains(next_dt) {
                next_idx = Some(idx)
            }
        }
        let next_idx = next_idx.unwrap();

        self.cur = (next_dt, next_idx);
        Some(ret)
    }
}

/// `RobotWorkTimeIterator` combines `TimeSegmentsIterator` and `BreakIterator`, and produces a finite sequence of time points.
#[derive(Eq, PartialEq, Debug)]
pub struct RobotWorkTimeIterator {
    cur: (NaiveDateTime, Option<usize>),
    end: NaiveDateTime,
    time_ranges_iter: TimeRangesIterator,
    break_iter: BreakIterator,
    breaking: Option<(NaiveDateTime, Option<usize>)>,
    is_finish: bool,
}

impl Iterator for RobotWorkTimeIterator {
    type Item = (NaiveDateTime, Option<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.cur;
        if self.is_finish { return None; }
        if ret.0 >= self.end {
            self.is_finish = true;
            return Some((self.end, None));
        }

        if let Some((break_end, mut end_status)) = self.breaking.take() {
            let mut time_ranges_iter = self.time_ranges_iter.clone();
            let (next_time_seg, next_status) = time_ranges_iter.next().unwrap();
            if next_time_seg <= break_end {
                end_status = Some(next_status);
                self.time_ranges_iter.next();
            }
            self.cur = (break_end, end_status);
            return Some(ret);
        }

        let mut time_ranges_iter = self.time_ranges_iter.clone();
        let (next_time_seg, next_status) = time_ranges_iter.next().unwrap();

        let mut break_iter = self.break_iter.clone();
        let (break_begin, break_end) = break_iter.next().unwrap();

        if next_time_seg < break_begin {
            self.cur = (next_time_seg, Some(next_status));
            self.time_ranges_iter.next();
        } else {
            self.cur = (break_begin, None);
            self.breaking = Some((break_end, ret.1));
            self.break_iter.next();
        }

        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn weekend() -> impl Iterator<Item=Weekday> {
        use Weekday::*;
        vec![Sat, Sun].into_iter()
    }

    fn weekday() -> impl Iterator<Item=Weekday> {
        use Weekday::*;
        vec![Mon, Tue, Wed, Thu, Fri].into_iter()
    }

    #[test]
    fn robot_work_time_iter_test() {
        let t = RobotWorkTime::new(
            NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
            NaiveDateTime::from_str("2021-09-06T12:59:00").unwrap(),
            vec![
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekend()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekend()),
            ],
        );

        let mut time_ranges_iter = TimeRangesIterator::new(
            NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
            vec![
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekend()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekend()),
            ],
        ).unwrap();
        time_ranges_iter.next();

        let mut it = RobotWorkTimeIterator {
            cur: (NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), Some(2)),
            end: NaiveDateTime::from_str("2021-09-06T12:59:00").unwrap(),
            break_iter: BreakIterator {
                start: NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
                work_duration: Duration::hours(8),
                rest_duration: Duration::hours(1),
            },
            breaking: None,
            time_ranges_iter,
            is_finish: false,
        };

        assert_eq!(t.into_iter(), it);

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T23:00:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T00:00:00").unwrap(), Some(1))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T06:00:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap(), Some(0))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T12:59:00").unwrap(), None)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn robot_work_time_iter_test_start_early() {
        let t = RobotWorkTime::new(
            NaiveDateTime::from_str("2021-09-10T00:01:00").unwrap(),
            NaiveDateTime::from_str("2021-09-12T00:30:00").unwrap(),
            vec![
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekend()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekend()),
            ],
        );
        let mut it = t.into_iter();

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T00:01:00").unwrap(), Some(1))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T07:00:00").unwrap(), Some(0))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T08:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T09:01:00").unwrap(), Some(0))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T17:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T18:01:00").unwrap(), Some(0))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T23:00:00").unwrap(), Some(1))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T00:00:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T02:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T03:01:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T07:00:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T11:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T12:01:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T20:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T21:01:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T23:00:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-12T00:00:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-12T00:30:00").unwrap(), None)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn robot_work_time_iter_test_start_late() {
        let t = RobotWorkTime::new(
            NaiveDateTime::from_str("2021-09-10T23:01:00").unwrap(),
            NaiveDateTime::from_str("2021-09-11T12:55:00").unwrap(),
            vec![
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekend()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekend()),
            ],
        );
        let mut it = t.into_iter();

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T23:01:00").unwrap(), Some(1))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T00:00:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T07:00:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T07:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T08:01:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T12:55:00").unwrap(), None)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn robot_work_time_iter_test_complex_scheme() {
        let t = RobotWorkTime::new(
            NaiveDateTime::from_str("2021-09-10T23:01:00").unwrap(),
            NaiveDateTime::from_str("2021-09-11T20:55:00").unwrap(),
            vec![
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(3, 0, 0), NaiveTime::from_hms(15, 0, 0)), weekend()),
                TimeRange::new((NaiveTime::from_hms(15, 0, 0), NaiveTime::from_hms(3, 0, 0)), weekend()),
            ],
        );
        let mut it = t.into_iter();

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T23:01:00").unwrap(), Some(1))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T00:00:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T03:00:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T07:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T08:01:00").unwrap(), Some(2))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T15:00:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T16:01:00").unwrap(), None)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T17:01:00").unwrap(), Some(3))));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T20:55:00").unwrap(), None)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn time_seg_iter_test() {
        let mut it = TimeRangesIterator::new(
            NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
            vec![
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekend()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekend()),
            ],
        ).unwrap();

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), 2)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T23:00:00").unwrap(), 3)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T00:00:00").unwrap(), 1)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap(), 0)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T23:00:00").unwrap(), 1)));
    }

    #[test]
    fn break_iter_test() {
        let mut it = BreakIterator {
            start: NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
            work_duration: Duration::hours(8),
            rest_duration: Duration::hours(1),
        };

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T06:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap())));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T15:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T16:00:00").unwrap())));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-07T00:00:00").unwrap(), NaiveDateTime::from_str("2021-09-07T01:00:00").unwrap())));
    }

    #[test]
    fn integration_test_2() {
        let t = RobotWorkTime::new(
            NaiveDateTime::from_str("2038-01-11T07:00:00").unwrap(),
            NaiveDateTime::from_str("2038-01-17T19:00:00").unwrap(),
            vec![
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekday()),
                TimeRange::new((NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)), weekend()),
                TimeRange::new((NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)), weekend()),
            ],
        );
        let s = t.clone().into_iter().zip(std::iter::once((NaiveDateTime::from_str("2038-01-11T07:00:00").unwrap(), None)).chain(t.into_iter())).skip(1)
            .fold(vec![Duration::zero(); 4], |mut acc, ((e, _), (s, status))| {
                if let Some(idx) = status {
                    acc[idx] = acc[idx] + (e - s);
                }
                acc
            });
        let res = s[0].num_minutes() * 20 + s[1].num_minutes() * 25 + s[2].num_minutes() * 30 + s[3].num_minutes() * 35;
        assert_eq!(res, 202200);
    }
}