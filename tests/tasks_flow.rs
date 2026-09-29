mod common;

use common::*;
use reqwest::StatusCode;
use serde_json::{json, Value};

#[tokio::test]
async fn admin_creates_five_tasks_and_assigns_three() {
    let app = spawn_app().await;
    app.seed().await;
    let admin = app.token_for(ADMIN_EMAIL, ADMIN_PASSWORD).await;

    let ids = app.create_five_tasks(&admin).await;
    assert_eq!(ids.len(), 5);

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tasks")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 5);

    let res = app.assign(&admin, &ids[..3], BOND_EMAIL).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["assigned"], 3);

    let (assigned,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM tasks WHERE assigned_to_id IS NOT NULL")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(assigned, 3);
}

#[tokio::test]
async fn assign_unknown_task_or_email_is_404() {
    let app = spawn_app().await;
    app.seed().await;
    let admin = app.token_for(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let ids = app.create_five_tasks(&admin).await;

    let unknown_task = app
        .assign(
            &admin,
            &[ids[0].clone(), uuid::Uuid::new_v4().to_string()],
            BOND_EMAIL,
        )
        .await;
    assert_eq!(unknown_task.status(), StatusCode::NOT_FOUND);

    let unknown_email = app.assign(&admin, &ids[..1], "nobody@example.com").await;
    assert_eq!(unknown_email.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn staff_cannot_create_or_assign() {
    let app = spawn_app().await;
    app.seed().await;
    let admin = app.token_for(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let ids = app.create_five_tasks(&admin).await;
    let bond = app.token_for(BOND_EMAIL, BOND_PASSWORD).await;

    let create = app
        .post(
            "/tasks",
            &json!({ "title": "x", "description": "x", "status": "todo", "priority": "low" }),
            Some(&bond),
        )
        .await;
    assert_eq!(create.status(), StatusCode::FORBIDDEN);

    let assign = app.assign(&bond, &ids[..1], BOND_EMAIL).await;
    assert_eq!(assign.status(), StatusCode::FORBIDDEN);

    let patch = app
        .patch(
            &format!("/tasks/{}", ids[0]),
            &json!({ "title": "hacked" }),
            Some(&bond),
        )
        .await;
    assert_eq!(patch.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn view_my_tasks_returns_exactly_the_assigned_rows_then_hits_cache() {
    let app = spawn_app().await;
    app.seed().await;
    let admin = app.token_for(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let ids = app.create_five_tasks(&admin).await;
    app.assign(&admin, &ids[..3], BOND_EMAIL).await;
    let bond = app.token_for(BOND_EMAIL, BOND_PASSWORD).await;

    let first = app.view(&bond).await;
    assert_eq!(first["user"]["email"], BOND_EMAIL);
    assert_eq!(first["user"]["role"], "staff");
    assert_eq!(first["summary"]["total_assigned_tasks"], 3);
    assert_eq!(first["cache"]["hit"], false);

    let tasks = first["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 3);
    let got_ids: Vec<&str> = tasks.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert_eq!(
        got_ids,
        ids[..3].iter().map(String::as_str).collect::<Vec<_>>()
    );
    let priorities: Vec<&str> = tasks
        .iter()
        .map(|t| t["priority"].as_str().unwrap())
        .collect();
    assert_eq!(priorities, ["high", "medium", "low"]);
    for t in tasks {
        assert_eq!(t["assigned_to"], BOND_EMAIL);
        assert!(t.get("description").is_none());
        assert!(t.get("created_by_id").is_none());
        assert!(t.get("created_at").is_none());
    }

    // Matches the database exactly.
    let db_rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT t.id FROM tasks t JOIN users u ON u.id = t.assigned_to_id
         WHERE u.email = $1 ORDER BY t.created_at, t.id",
    )
    .bind(BOND_EMAIL)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    let db_ids: Vec<String> = db_rows.iter().map(|(id,)| id.to_string()).collect();
    assert_eq!(db_ids, ids[..3]);

    // Second call is served from Redis.
    let key = task_view_key(&app.user_id(BOND_EMAIL).await);
    assert!(app.redis_key_exists(&key).await);
    let second = app.view(&bond).await;
    assert_eq!(second["cache"]["hit"], true);
    let mut a = first.clone();
    let mut b = second.clone();
    a["cache"].take();
    b["cache"].take();
    assert_eq!(a, b);
}

#[tokio::test]
async fn cached_view_is_stale_after_raw_sql_update() {
    let app = spawn_app().await;
    app.seed().await;
    let admin = app.token_for(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let ids = app.create_five_tasks(&admin).await;
    app.assign(&admin, &ids[..3], BOND_EMAIL).await;
    let bond = app.token_for(BOND_EMAIL, BOND_PASSWORD).await;

    let first = app.view(&bond).await;
    assert_eq!(first["cache"]["hit"], false);
    assert_eq!(first["tasks"][0]["title"], "Review dossier");

    // Bypass the API entirely.
    sqlx::query("UPDATE tasks SET title = 'Changed behind the cache' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&ids[0]).unwrap())
        .execute(&app.pool)
        .await
        .unwrap();

    let second = app.view(&bond).await;
    assert_eq!(second["cache"]["hit"], true);
    assert_eq!(
        second["tasks"][0]["title"], "Review dossier",
        "the handler must not have read Postgres again"
    );
}

#[tokio::test]
async fn patch_invalidates_cache_and_next_view_shows_new_title() {
    let app = spawn_app().await;
    app.seed().await;
    let admin = app.token_for(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let ids = app.create_five_tasks(&admin).await;
    app.assign(&admin, &ids[..3], BOND_EMAIL).await;
    let bond = app.token_for(BOND_EMAIL, BOND_PASSWORD).await;
    let key = task_view_key(&app.user_id(BOND_EMAIL).await);

    app.view(&bond).await;
    assert!(app.redis_key_exists(&key).await);

    let res = app
        .patch(
            &format!("/tasks/{}", ids[0]),
            &json!({ "title": "Review dossier (v2)" }),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["title"], "Review dossier (v2)");
    assert!(
        !app.redis_key_exists(&key).await,
        "patch must delete the key"
    );

    let view = app.view(&bond).await;
    assert_eq!(view["cache"]["hit"], false);
    assert_eq!(view["tasks"][0]["title"], "Review dossier (v2)");

    // Empty patch is a 400.
    let empty = app
        .patch(&format!("/tasks/{}", ids[0]), &json!({}), Some(&admin))
        .await;
    assert_eq!(empty.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reassigning_deletes_both_users_keys() {
    let app = spawn_app().await;
    app.seed().await;
    let admin = app.token_for(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let ids = app.create_five_tasks(&admin).await;
    app.assign(&admin, &ids[..3], BOND_EMAIL).await;

    let bond = app.token_for(BOND_EMAIL, BOND_PASSWORD).await;
    let bond_key = task_view_key(&app.user_id(BOND_EMAIL).await);
    let admin_key = task_view_key(&app.user_id(ADMIN_EMAIL).await);

    // Warm both caches.
    app.view(&bond).await;
    app.view(&admin).await;
    assert!(app.redis_key_exists(&bond_key).await);
    assert!(app.redis_key_exists(&admin_key).await);

    // Move one of Bond's tasks to Admin.
    let res = app.assign(&admin, &ids[..1], ADMIN_EMAIL).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        !app.redis_key_exists(&bond_key).await,
        "previous assignee key"
    );
    assert!(!app.redis_key_exists(&admin_key).await, "new assignee key");

    let bond_view = app.view(&bond).await;
    assert_eq!(bond_view["cache"]["hit"], false);
    assert_eq!(bond_view["summary"]["total_assigned_tasks"], 2);
    let admin_view = app.view(&admin).await;
    assert_eq!(admin_view["summary"]["total_assigned_tasks"], 1);
    assert_eq!(admin_view["tasks"][0]["id"], ids[0]);

    // PATCH with assignee_email also clears both keys.
    let res = app
        .patch(
            &format!("/tasks/{}", ids[0]),
            &json!({ "assignee_email": BOND_EMAIL }),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!app.redis_key_exists(&bond_key).await);
    assert!(!app.redis_key_exists(&admin_key).await);
}
