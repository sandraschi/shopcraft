//! Daily sales totals. Each shop queries its own book. Quiet days are filled
//! in only when the span is short enough to draw as a calendar.

use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesPoint {
    pub day: String,
    pub cents: i64,
    pub orders: i64,
}

pub fn by_day(conn: &Connection, sql: &str) -> rusqlite::Result<Vec<SalesPoint>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(SalesPoint {
            day: row.get(0)?,
            cents: row.get(1)?,
            orders: row.get(2)?,
        })
    })?;
    let points = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(include_quiet_days(points))
}

fn include_quiet_days(points: Vec<SalesPoint>) -> Vec<SalesPoint> {
    let Some(first) = points.first() else {
        return points;
    };
    let Some(last) = points.last() else {
        return points;
    };
    let Ok(start) = NaiveDate::parse_from_str(&first.day, "%Y-%m-%d") else {
        return points;
    };
    let Ok(end) = NaiveDate::parse_from_str(&last.day, "%Y-%m-%d") else {
        return points;
    };
    let span = (end - start).num_days();
    if !(1..=92).contains(&span) {
        return points;
    }
    let mut out = Vec::new();
    let mut index = 0usize;
    let mut day = start;
    while day <= end {
        let key = day.format("%Y-%m-%d").to_string();
        if points.get(index).is_some_and(|point| point.day == key) {
            out.push(points[index].clone());
            index += 1;
        } else {
            out.push(SalesPoint {
                day: key,
                cents: 0,
                orders: 0,
            });
        }
        day += Duration::days(1);
    }
    out
}
