//! Shared SQL row -> [`IndexedNode`] mapping used by every query module.

use crate::model::{IndexedNode, TimestampRecord};
use chrono::{NaiveDate, NaiveTime};
use sqlx::sqlite::SqliteRow;
use sqlx::Row;

pub const NODE_JOIN_FILE: &str = "\
    SELECT n.id, n.file_id, f.path AS file_path, f.content_hash AS version,
           n.parent_id, n.org_id, n.level, n.position, n.title,
           n.todo_state, n.todo_type, n.priority, n.tags,
           n.scheduled_date, n.scheduled_time, n.scheduled_repeater, n.scheduled_active,
           n.deadline_date, n.deadline_time, n.deadline_repeater, n.deadline_active,
           n.closed_date, n.properties_ordered, n.body
    FROM nodes n JOIN files f ON n.file_id = f.id";

fn timestamp_of(
    row: &SqliteRow,
    date_col: &str,
    time_col: &str,
    repeater_col: &str,
    active_col: &str,
) -> Result<Option<TimestampRecord>, sqlx::Error> {
    let date: Option<String> = row.try_get(date_col)?;
    let Some(date) = date else { return Ok(None) };
    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
    let time: Option<String> = row.try_get(time_col)?;
    let time = time
        .map(|t| NaiveTime::parse_from_str(&t, "%H:%M:%S"))
        .transpose()
        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
    let repeater: Option<String> = row.try_get(repeater_col)?;
    let active: Option<i64> = row.try_get(active_col)?;
    Ok(Some(TimestampRecord {
        date,
        time,
        active: active.unwrap_or(1) != 0,
        repeater,
    }))
}

pub fn map_row(row: &SqliteRow) -> Result<IndexedNode, sqlx::Error> {
    let tags_json: String = row.try_get("tags")?;
    let props_json: String = row.try_get("properties_ordered")?;
    Ok(IndexedNode {
        id: row.try_get("id")?,
        org_id: row.try_get("org_id")?,
        file_id: row.try_get("file_id")?,
        file_path: row.try_get("file_path")?,
        parent_id: row.try_get("parent_id")?,
        level: row.try_get("level")?,
        position: row.try_get("position")?,
        title: row.try_get("title")?,
        todo_state: row.try_get("todo_state")?,
        todo_type: row.try_get("todo_type")?,
        priority: row.try_get("priority")?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        scheduled: timestamp_of(
            row,
            "scheduled_date",
            "scheduled_time",
            "scheduled_repeater",
            "scheduled_active",
        )?,
        deadline: timestamp_of(
            row,
            "deadline_date",
            "deadline_time",
            "deadline_repeater",
            "deadline_active",
        )?,
        closed: {
            let closed_date: Option<String> = row.try_get("closed_date")?;
            closed_date
                .map(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d"))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .map(|date| TimestampRecord {
                    date,
                    time: None,
                    active: false,
                    repeater: None,
                })
        },
        properties: serde_json::from_str(&props_json).unwrap_or_default(),
        body: row.try_get("body")?,
        version: row.try_get("version")?,
    })
}
