pub mod schema;

use chrono::{Datelike, Duration, NaiveDateTime, NaiveTime};

/// `RobotStatus` encodes the different status of the robot.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum RobotStatus {
    StandardDay = 1,
    ExtraDay = 2,
    StandardNight = 3,
    ExtraNight = 6,
    Breaking = 0,
    Finish = 10
}

impl RobotStatus {
    fn from_u32(i: u32) -> Option<Self> {
        match i {
            x if x == Self::StandardDay as u32 => { Some(Self::StandardDay) }
            x if x == Self::StandardNight as u32 => { Some(Self::StandardNight) }
            x if x == Self::ExtraDay as u32 => { Some(Self::ExtraDay) }
            x if x == Self::ExtraNight as u32 => { Some(Self::ExtraNight) }
            x if x == Self::Breaking as u32 => { Some(Self::StandardDay) }
            _ => { None }
        }
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
    time_segments: Vec<(NaiveTime, NaiveTime)>,
}

impl RobotWorkTime {
    pub fn new(start: NaiveDateTime, end: NaiveDateTime, time_segments: Vec<(NaiveTime, NaiveTime)>) -> Self {
        Self { start, end, time_segments }
    }

    pub fn into_iter(self) -> RobotWorkTimeIterator {
        let RobotWorkTime { time_segments, start, end: _ } = self;
        let time_segs_to_robot_status = vec![RobotStatus::StandardDay, RobotStatus::StandardNight];

        let t = start.time();
        let d = start.date();
        let mut robot_status = RobotStatus::Breaking;
        let time_segments = time_segments.into_iter().enumerate().map(|(idx, (s, e))| {
            if s < e {
                if t >= s && t < e {
                    robot_status = time_segs_to_robot_status[idx];
                    (NaiveDateTime::new(d + Duration::days(1), s), NaiveDateTime::new(d + Duration::days(1), e))
                } else if t < s {
                    (NaiveDateTime::new(d, s), NaiveDateTime::new(d, e))
                } else {
                    (NaiveDateTime::new(d + Duration::days(1), s), NaiveDateTime::new(d + Duration::days(1), e))
                }
            } else {
                if t >= s || t < e {
                    robot_status = time_segs_to_robot_status[idx];
                    if t >= s {
                        (NaiveDateTime::new(d + Duration::days(1), s), NaiveDateTime::new(d + Duration::days(2), e))
                    } else {
                        (NaiveDateTime::new(d, s), NaiveDateTime::new(d + Duration::days(1), e))
                    }
                } else {
                    (NaiveDateTime::new(d, s), NaiveDateTime::new(d + Duration::days(1), e))
                }
            }
        }).collect::<Vec<_>>();
        use chrono::Weekday::*;
        match start.date().weekday() {
            Sat | Sun => { robot_status = RobotStatus::from_u32(robot_status as u32 * 2).unwrap() }
            _ => {}
        }

        let mut time_segments_iter = TimeSegmentsIterator {
            cur: (start, robot_status),
            time_segments
        };
        time_segments_iter.next();

        let break_iter = BreakIterator {
            start: self.start,
            work_duration: Duration::hours(8),
            rest_duration: Duration::hours(1),
        };

        RobotWorkTimeIterator {
            cur: (self.start, robot_status),
            end: self.end,
            time_segments_iter,
            break_iter,
            breaking: None,
            is_finish: false
        }
    }
}

/// `TimeSegmentsIterator` produces a infinite sequence of time points, at which the robot status (and the corresponding rates) changed.
#[derive(Eq, PartialEq, Debug, Clone)]
struct TimeSegmentsIterator {
    cur: (NaiveDateTime, RobotStatus),
    time_segments: Vec<(NaiveDateTime, NaiveDateTime)>,
}

impl Iterator for TimeSegmentsIterator {
    type Item = (NaiveDateTime, RobotStatus);

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.cur.clone();
        let (date_time, _status) = self.cur.clone();

        let (min_idx, _) = self.time_segments.iter_mut().enumerate().min_by_key(|(_idx, seg)| (**seg).0).unwrap();
        let mut new_cur_dt = self.time_segments[min_idx].0;

        let time_segs_to_robot_status = vec![RobotStatus::StandardDay, RobotStatus::StandardNight];
        let mut new_status = time_segs_to_robot_status[min_idx];
        use chrono::Weekday::*;
        match new_cur_dt.date().weekday() {
            Sat | Sun => {
                new_status = RobotStatus::from_u32(new_status as u32 * 2).unwrap()
            }
            _ => {}
        }

        if date_time.date().weekday() == Sun && new_cur_dt.date().weekday() == Mon {
            new_cur_dt = new_cur_dt.date().and_hms(0, 0, 0);
            new_status = RobotStatus::StandardNight;
            self.cur = (new_cur_dt, new_status);
            return Some(ret);
        }
        if date_time.date().weekday() == Fri && new_cur_dt.date().weekday() == Sat {
            new_cur_dt = new_cur_dt.date().and_hms(0, 0, 0);
            new_status = RobotStatus::ExtraNight;
            self.cur = (new_cur_dt, new_status);
            return Some(ret);
        }

        self.cur = (new_cur_dt, new_status);
        self.time_segments[min_idx].0 += Duration::days(1);
        self.time_segments[min_idx].1 += Duration::days(1);

        return Some(ret);
    }
}

/// `RobotWorkTimeIterator` combines `TimeSegmentsIterator` and `BreakIterator`, and produces a finite sequence of time points.
#[derive(Eq, PartialEq, Debug)]
pub struct RobotWorkTimeIterator {
    cur: (NaiveDateTime, RobotStatus),
    end: NaiveDateTime,
    time_segments_iter: TimeSegmentsIterator,
    break_iter: BreakIterator,
    breaking: Option<(NaiveDateTime, RobotStatus)>,
    is_finish: bool
}

impl Iterator for RobotWorkTimeIterator {
    type Item = (NaiveDateTime, RobotStatus);

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.cur.clone();
        if self.is_finish { return None; }
        if ret.0 >= self.end { self.is_finish = true; return Some((self.end, RobotStatus::Finish)); }

        if let Some((break_end, mut end_status)) = self.breaking.take() {
            let mut time_segments_iter = self.time_segments_iter.clone();
            let (next_time_seg, next_status) = time_segments_iter.next().unwrap().clone();
            if next_time_seg <= break_end {
                end_status = next_status;
                self.time_segments_iter.next();
            }
            self.cur = (break_end, end_status);
            return Some(ret);
        }

        let mut time_segments_iter = self.time_segments_iter.clone();
        let (next_time_seg, next_status) = time_segments_iter.next().unwrap().clone();

        let mut break_iter = self.break_iter.clone();
        let (break_begin, break_end) = break_iter.next().unwrap().clone();

        if next_time_seg < break_begin {
            self.cur = (next_time_seg, next_status);
            self.time_segments_iter.next();
        } else {
            self.cur = (break_begin, RobotStatus::Breaking);
            self.breaking = Some((break_end, ret.1));
            self.break_iter.next();
        }

        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn robot_work_time_iter_test() {
        let t = RobotWorkTime::new(NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
                                   NaiveDateTime::from_str("2021-09-06T12:59:00").unwrap(),
                                   vec![(NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)),
                                        (NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)),
                                   ]
        );

        let mut time_segments_iter = TimeSegmentsIterator {
            cur: (NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), RobotStatus::ExtraDay),
            time_segments: vec![(NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T23:00:00").unwrap()),
                                (NaiveDateTime::from_str("2021-09-05T23:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap())]
        };
        time_segments_iter.next();
        let mut it = RobotWorkTimeIterator {
            cur: (NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), RobotStatus::ExtraDay),
            end: NaiveDateTime::from_str("2021-09-06T12:59:00").unwrap(),
            break_iter: BreakIterator {
                start: NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
                work_duration: Duration::hours(8),
                rest_duration: Duration::hours(1),
            },
            breaking: None,
            time_segments_iter,
            is_finish: false
        };

        assert_eq!(t.into_iter(), it);

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), RobotStatus::ExtraDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T23:00:00").unwrap(), RobotStatus::ExtraNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T00:00:00").unwrap(), RobotStatus::StandardNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T06:00:00").unwrap(), RobotStatus::Breaking)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap(), RobotStatus::StandardDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T12:59:00").unwrap(), RobotStatus::Finish)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn robot_work_time_iter_test_start_early() {
        let t = RobotWorkTime::new(NaiveDateTime::from_str("2021-09-10T00:01:00").unwrap(),
                                   NaiveDateTime::from_str("2021-09-12T00:30:00").unwrap(),
                                   vec![(NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)),
                                        (NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)),
                                   ]
        );
        let mut it = t.into_iter();

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T00:01:00").unwrap(), RobotStatus::StandardNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T07:00:00").unwrap(), RobotStatus::StandardDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T08:01:00").unwrap(), RobotStatus::Breaking)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T09:01:00").unwrap(), RobotStatus::StandardDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T17:01:00").unwrap(), RobotStatus::Breaking)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T18:01:00").unwrap(), RobotStatus::StandardDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T23:00:00").unwrap(), RobotStatus::StandardNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T00:00:00").unwrap(), RobotStatus::ExtraNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T02:01:00").unwrap(), RobotStatus::Breaking)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T03:01:00").unwrap(), RobotStatus::ExtraNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T07:00:00").unwrap(), RobotStatus::ExtraDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T11:01:00").unwrap(), RobotStatus::Breaking)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T12:01:00").unwrap(), RobotStatus::ExtraDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T20:01:00").unwrap(), RobotStatus::Breaking)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T21:01:00").unwrap(), RobotStatus::ExtraDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T23:00:00").unwrap(), RobotStatus::ExtraNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-12T00:30:00").unwrap(), RobotStatus::Finish)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn robot_work_time_iter_test_start_late() {
        let t = RobotWorkTime::new(NaiveDateTime::from_str("2021-09-10T23:01:00").unwrap(),
                                   NaiveDateTime::from_str("2021-09-11T12:55:00").unwrap(),
                                   vec![(NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)),
                                        (NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0)),
                                   ]
        );
        let mut it = t.into_iter();

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-10T23:01:00").unwrap(), RobotStatus::StandardNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T00:00:00").unwrap(), RobotStatus::ExtraNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T07:00:00").unwrap(), RobotStatus::ExtraDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T07:01:00").unwrap(), RobotStatus::Breaking)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T08:01:00").unwrap(), RobotStatus::ExtraDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-11T12:55:00").unwrap(), RobotStatus::Finish)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn time_seg_iter_test() {
        let mut it = TimeSegmentsIterator {
            cur: (NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), RobotStatus::ExtraDay),
            time_segments: vec![(NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T23:00:00").unwrap()),
                                (NaiveDateTime::from_str("2021-09-05T23:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap())]
        };

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(), RobotStatus::ExtraDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-05T23:00:00").unwrap(), RobotStatus::ExtraNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T00:00:00").unwrap(), RobotStatus::StandardNight)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap(), RobotStatus::StandardDay)));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T23:00:00").unwrap(), RobotStatus::StandardNight)));
    }

    #[test]
    fn break_iter_test() {
        let mut it = BreakIterator {
            start: NaiveDateTime::from_str("2021-09-05T22:00:00").unwrap(),
            work_duration: Duration::hours(8),
            rest_duration: Duration::hours(1)
        };

        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T06:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T07:00:00").unwrap())));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-06T15:00:00").unwrap(), NaiveDateTime::from_str("2021-09-06T16:00:00").unwrap())));
        assert_eq!(it.next(), Some((NaiveDateTime::from_str("2021-09-07T00:00:00").unwrap(), NaiveDateTime::from_str("2021-09-07T01:00:00").unwrap())));
    }

    #[test]
    fn integration_test_2() {
        let t = RobotWorkTime::new(NaiveDateTime::from_str("2038-01-11T07:00:00").unwrap(),
                                   NaiveDateTime::from_str("2038-01-17T19:00:00").unwrap(),
                                   vec![(NaiveTime::from_hms(7, 0, 0), NaiveTime::from_hms(23, 0, 0)),
                                        (NaiveTime::from_hms(23, 0, 0), NaiveTime::from_hms(7, 0, 0))]
        );
        let s = t.clone().into_iter().zip(std::iter::once((NaiveDateTime::from_str("2038-01-11T07:00:00").unwrap(), RobotStatus::Finish)).chain(t.into_iter())).skip(1)
            .fold(vec![Duration::zero(); 10], |mut acc, ((e, _), (s, status))| {
                acc[status as usize] = acc[status as usize] + (e - s);
                acc
            });
        let res = s[1].num_minutes()*20 + s[2].num_minutes()*30 + s[3].num_minutes()*25 + s[6].num_minutes()*35;
        assert_eq!(res, 202200);
    }
}