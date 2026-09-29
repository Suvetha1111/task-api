mod common;

use common::*;
use reqwest::StatusCode;
use serde_json::{json, Value};

#[tokio::test]
async fn seed_is_idempotent() {
    let app = spawn_app().await;

    let first = app.seed().await;
    assert_eq!(first["created"], json!([ADMIN_EMAIL, BOND_EMAIL]));
    assert_eq!(first["existing"], json!([]));

    let second = app.seed().await;
    assert_eq!(second["created"], json!([]));
    assert_eq!(second["existing"], json!([ADMIN_EMAIL, BOND_EMAIL]));

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 2);

    // No password material in any response body.
    let text = serde_json::to_string(&second).unwrap();
    assert!(!text.contains("password"));
    assert!(!text.contains("$2b$"));
}

#[tokio::test]
async fn login_returns_challenge_and_no_jwt() {
    let app = spawn_app().await;
    app.seed().await;

    let res = app
        .post(
            "/auth/login",
            &json!({ "email": ADMIN_EMAIL, "password": ADMIN_PASSWORD }),
            None,
        )
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert!(body["login_challenge_id"].is_string());
    for key in ["access_token", "token", "jwt"] {
        assert!(body.get(key).is_none(), "login body must not contain {key}");
    }

    // The challenge row stores only a bcrypt hash, never the raw code.
    let (code_hash,): (String,) = sqlx::query_as("SELECT code_hash FROM login_challenges")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert!(code_hash.starts_with("$2"));
    let (raw,): (String,) = sqlx::query_as("SELECT verification_code FROM email_logs")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_ne!(code_hash, raw);
}

#[tokio::test]
async fn wrong_credentials_are_401_without_revealing_field() {
    let app = spawn_app().await;
    app.seed().await;

    let bad_password = app
        .post(
            "/auth/login",
            &json!({ "email": ADMIN_EMAIL, "password": "nope" }),
            None,
        )
        .await;
    let bad_email = app
        .post(
            "/auth/login",
            &json!({ "email": "ghost@example.com", "password": ADMIN_PASSWORD }),
            None,
        )
        .await;
    assert_eq!(bad_password.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(bad_email.status(), StatusCode::UNAUTHORIZED);
    let a: Value = bad_password.json().await.unwrap();
    let b: Value = bad_email.json().await.unwrap();
    assert_eq!(a["message"], b["message"]);
}

#[tokio::test]
async fn correct_code_returns_jwt() {
    let app = spawn_app().await;
    app.seed().await;

    let (challenge, code) = app.login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let res = app.verify(&challenge, &code).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert!(body["access_token"].is_string());
    assert_eq!(body["token_type"], "Bearer");

    // The token works on a protected route.
    let me = app
        .get(
            "/tasks/view-my-tasks",
            Some(body["access_token"].as_str().unwrap()),
        )
        .await;
    assert_eq!(me.status(), StatusCode::OK);
}

#[tokio::test]
async fn wrong_code_is_401_and_does_not_consume_challenge() {
    let app = spawn_app().await;
    app.seed().await;

    let (challenge, code) = app.login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let wrong = if code == "000000" { "111111" } else { "000000" };

    let res = app.verify(&challenge, wrong).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let body: Value = res.json().await.unwrap();
    assert!(body["message"].as_str().unwrap().contains("incorrect"));

    // Still usable with the right code.
    let res = app.verify(&challenge, &code).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn reused_code_is_401() {
    let app = spawn_app().await;
    app.seed().await;

    let (challenge, code) = app.login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    assert_eq!(app.verify(&challenge, &code).await.status(), StatusCode::OK);

    let res = app.verify(&challenge, &code).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let body: Value = res.json().await.unwrap();
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("already been used"));
}

#[tokio::test]
async fn expired_code_is_401_with_one_second_ttl() {
    let app = spawn_app_with_otp_ttl(1).await;
    app.seed().await;

    let (challenge, code) = app.login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let res = app.verify(&challenge, &code).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let body: Value = res.json().await.unwrap();
    assert!(body["message"].as_str().unwrap().contains("expired"));
}

#[tokio::test]
async fn protected_routes_reject_missing_and_invalid_tokens() {
    let app = spawn_app().await;
    app.seed().await;

    let missing = app.get("/tasks/view-my-tasks", None).await;
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

    let garbage = app.get("/tasks/view-my-tasks", Some("not.a.jwt")).await;
    assert_eq!(garbage.status(), StatusCode::UNAUTHORIZED);

    // A token for a user that no longer exists.
    let bond = app.token_for(BOND_EMAIL, BOND_PASSWORD).await;
    sqlx::query(
        "DELETE FROM login_challenges WHERE user_id = (SELECT id FROM users WHERE email = $1)",
    )
    .bind(BOND_EMAIL)
    .execute(&app.pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(BOND_EMAIL)
        .execute(&app.pool)
        .await
        .unwrap();
    let gone = app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(gone.status(), StatusCode::UNAUTHORIZED);

    // Public routes still work without a token.
    let swagger = app.get("/swagger-ui/", None).await;
    assert_eq!(swagger.status(), StatusCode::OK);
}
