//! Agenda queries: today / overdue / upcoming, mirroring org-agenda
//! semantics as closely as is practical for v0.1 (docs/mvp.md, §8 of the
//! project brief). Repeating entries are expanded via `recurrence.rs`;
//! overdue is computed from each entry's literal stored deadline date
//! (a repeating deadline's overdue status does not re-derive its "most
//! recent missed occurrence" in v0.1 — documented simplification).

use crate::model::{AgendaItem, AgendaKind};
use crate::recurrence::occurs_on;
use crate::row::{map_row, NODE_JOIN_FILE};
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct AgendaResult {
    pub today: Vec<AgendaItem>,
    pub overdue: Vec<AgendaItem>,
    pub upcoming: BTreeMap<NaiveDate, Vec<AgendaItem>>,
}

pub async fn agenda(
    pool: &SqlitePool,
    workspace_key: &str,
    today: NaiveDate,
    to: NaiveDate,
) -> Result<AgendaResult, sqlx::Error> {
    let sql = format!(
        "{NODE_JOIN_FILE} WHERE f.workspace_root = ?1 AND (n.scheduled_date IS NOT NULL OR n.deadline_date IS NOT NULL)"
    );
    let rows = sqlx::query(&sql).bind(workspace_key).fetch_all(pool).await?;

    let mut result = AgendaResult {
        today: Vec::new(),
        overdue: Vec::new(),
        upcoming: BTreeMap::new(),
    };

    for row in &rows {
        let node = map_row(row)?;

        if let Some(deadline) = &node.deadline {
            if deadline.date < today && node.todo_type.as_deref() == Some("todo") {
                result.overdue.push(AgendaItem {
                    node: node.clone(),
                    agenda_date: deadline.date,
                    agenda_kind: AgendaKind::Deadline,
                });
            }
        }

        let mut date = today;
        while date <= to {
            if let Some(scheduled) = &node.scheduled {
                if occurs_on(scheduled.date, scheduled.repeater.as_deref(), date) {
                    push_occurrence(&mut result, &node, date, today, AgendaKind::Scheduled);
                }
            }
            if let Some(deadline) = &node.deadline {
                if occurs_on(deadline.date, deadline.repeater.as_deref(), date) {
                    push_occurrence(&mut result, &node, date, today, AgendaKind::Deadline);
                }
            }
            date = date.succ_opt().unwrap();
        }
    }

    Ok(result)
}

fn push_occurrence(
    result: &mut AgendaResult,
    node: &crate::model::IndexedNode,
    date: NaiveDate,
    today: NaiveDate,
    kind: AgendaKind,
) {
    let item = AgendaItem {
        node: node.clone(),
        agenda_date: date,
        agenda_kind: kind,
    };
    if date == today {
        result.today.push(item);
    } else {
        result.upcoming.entry(date).or_default().push(item);
    }
}
