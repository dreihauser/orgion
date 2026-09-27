//! End-to-end test of the v0.1 "Initial Success Condition" from the
//! project brief (docs/mvp.md): parse -> index -> agenda -> edit via API
//! layer -> reindex -> observe the change, plus the reverse direction
//! (an external/"Emacs" edit is picked up on reindex).

use chrono::NaiveDate;
use org_storage::Workspace;

const TASKS_ORG: &str = "\
#+TODO: TODO NEXT | DONE

* TODO Build Orgion
DEADLINE: <2026-10-01 Thu>

* NEXT Implement parser

* DONE Create repository
";

async fn setup() -> (tempfile::TempDir, Workspace, sqlx::SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("tasks.org"), TASKS_ORG).unwrap();
    let workspace = Workspace::open(dir.path()).unwrap();
    let pool = org_index::connect("sqlite::memory:").await.unwrap();
    org_index::reindex_workspace(&pool, &workspace, "ws1")
        .await
        .unwrap();
    (dir, workspace, pool)
}

#[tokio::test]
async fn indexes_three_tasks_with_correct_todo_state() {
    let (_dir, _ws, pool) = setup().await;
    let files = org_index::queries::list_files(&pool, "ws1").await.unwrap();
    assert_eq!(files.len(), 1);
    let nodes = org_index::queries::list_nodes_for_file(&pool, &files[0].id)
        .await
        .unwrap();
    assert_eq!(nodes.len(), 3);
    assert_eq!(nodes[0].title, "Build Orgion");
    assert_eq!(nodes[0].todo_state.as_deref(), Some("TODO"));
    assert_eq!(nodes[1].todo_state.as_deref(), Some("NEXT"));
    assert_eq!(nodes[2].todo_state.as_deref(), Some("DONE"));
}

#[tokio::test]
async fn agenda_surfaces_the_deadline() {
    let (_dir, _ws, pool) = setup().await;
    let today = NaiveDate::from_ymd_opt(2026, 9, 28).unwrap();
    let to = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
    let result = org_index::query_agenda(&pool, "ws1", today, to).await.unwrap();
    let all: Vec<_> = result
        .today
        .iter()
        .chain(result.upcoming.values().flatten())
        .collect();
    assert!(all.iter().any(|item| item.node.title == "Build Orgion"));
}

#[tokio::test]
async fn web_ui_edit_writes_through_to_the_file() {
    let (_dir, ws, pool) = setup().await;
    let files = org_index::queries::list_files(&pool, "ws1").await.unwrap();
    let nodes = org_index::queries::list_nodes_for_file(&pool, &files[0].id)
        .await
        .unwrap();
    let build_orgion = nodes.iter().find(|n| n.title == "Build Orgion").unwrap();

    let updated = org_index::edit_node(
        &pool,
        &ws,
        "ws1",
        &build_orgion.id,
        &[org_parser::NodeEdit::SetTodoState(Some("DONE".to_string()))],
        &build_orgion.version,
    )
    .await
    .unwrap();

    assert_eq!(updated.todo_state.as_deref(), Some("DONE"));

    // The actual file on disk must reflect the change (source of truth).
    let rel = ws.rel_path("tasks.org").unwrap();
    let on_disk = ws.read(&rel).unwrap();
    assert!(on_disk.contents.contains("* DONE Build Orgion"));
    assert!(!on_disk.contents.contains("* TODO Build Orgion"));
    // Sibling headings untouched.
    assert!(on_disk.contents.contains("* NEXT Implement parser"));
}

#[tokio::test]
async fn stale_version_is_rejected_with_conflict() {
    let (_dir, ws, pool) = setup().await;
    let files = org_index::queries::list_files(&pool, "ws1").await.unwrap();
    let nodes = org_index::queries::list_nodes_for_file(&pool, &files[0].id)
        .await
        .unwrap();
    let node = nodes.iter().find(|n| n.title == "Build Orgion").unwrap();

    let err = org_index::edit_node(
        &pool,
        &ws,
        "ws1",
        &node.id,
        &[org_parser::NodeEdit::SetTodoState(Some("DONE".to_string()))],
        "sha256:not-the-real-version",
    )
    .await
    .unwrap_err();

    match err {
        org_index::EditError::Storage(org_storage::StorageError::Conflict { .. }) => {}
        other => panic!("expected a version conflict, got {other:?}"),
    }
}

#[tokio::test]
async fn external_emacs_style_edit_is_picked_up_on_reindex() {
    let (_dir, ws, pool) = setup().await;
    // Simulate Emacs changing TODO -> NEXT directly on disk.
    let rel = ws.rel_path("tasks.org").unwrap();
    let current = ws.read(&rel).unwrap();
    let edited = current.contents.replace("* TODO Build Orgion", "* NEXT Build Orgion");
    std::fs::write(ws.root().join("tasks.org"), edited).unwrap();

    org_index::reindex_workspace(&pool, &ws, "ws1").await.unwrap();

    let files = org_index::queries::list_files(&pool, "ws1").await.unwrap();
    let nodes = org_index::queries::list_nodes_for_file(&pool, &files[0].id)
        .await
        .unwrap();
    let node = nodes.iter().find(|n| n.title == "Build Orgion").unwrap();
    assert_eq!(node.todo_state.as_deref(), Some("NEXT"));
}

#[tokio::test]
async fn search_finds_by_title() {
    let (_dir, _ws, pool) = setup().await;
    let results = org_index::search(&pool, "ws1", "parser", &org_index::SearchFilters::default())
        .await
        .unwrap();
    assert!(results.iter().any(|r| r.node.title == "Implement parser"));
}

#[tokio::test]
async fn rebuild_index_from_scratch_is_idempotent() {
    let (_dir, ws, pool) = setup().await;
    org_index::reindex_workspace(&pool, &ws, "ws1").await.unwrap();
    org_index::reindex_workspace(&pool, &ws, "ws1").await.unwrap();
    let files = org_index::queries::list_files(&pool, "ws1").await.unwrap();
    assert_eq!(files.len(), 1);
    let nodes = org_index::queries::list_nodes_for_file(&pool, &files[0].id)
        .await
        .unwrap();
    assert_eq!(nodes.len(), 3);
}
