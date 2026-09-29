//! Test harness: boots the real application on a random port against the
//! Postgres and Redis instances named by `TEST_DATABASE_URL` / `REDIS_URL`.
//! Nothing is mocked.
#![allow(dead_code)]

use std::net::TcpListener;

use actix_web::{App, HttpServer};
use deadpool_redis::redis::AsyncCommands;
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::sync::{Mutex, MutexGuard};

use task_api::app::{build_state, configure};
use task_api::config::Config;

/// All tests share one database, so they run one at a time.
static DB_LOCK: Mutex<()> = Mutex::const_new(());

pub const ADMIN_EMAIL: &str = "admin@example.com";
pub const ADMIN_PASSWORD: &str = "AdminPass123!";
pub const BOND_EMAIL: &str = "jamesbond@example.com";
pub const BOND_PASSWORD: &str = "BondPass123!";

pub struct TestApp {
    pub base_url: String,
    pub client: Client,
    pub pool: PgPool,
    pub redis: deadpool_redis::Pool,
    _guard: MutexGuard<'static, ()>,
}

fn test_config(otp_ttl_seconds: u64) -> Config {
    dotenv::dotenv().ok();
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://task:task@localhost:5432/taskdb_test".to_string());
    Config {
        database_url,
        redis_url: std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
        jwt_secret: "test-secret".to_string(),
        jwt_expiry_secs: 3600,
        otp_ttl_seconds,
        app_env: "development".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
    }
}

/// Boot the app with the default OTP TTL (300 s).
pub async fn spawn_app() -> TestApp {
    spawn_app_with_otp_ttl(300).await
}

pub async fn spawn_app_with_otp_ttl(otp_ttl_seconds: u64) -> TestApp {
    let guard = DB_LOCK.lock().await;
    let config = test_config(otp_ttl_seconds);

    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("connect to test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    sqlx::query("TRUNCATE email_logs, login_challenges, tasks, users CASCADE")
        .execute(&pool)
        .await
        .expect("truncate tables");

    let redis = deadpool_redis::Config::from_url(&config.redis_url)
        .create_pool(Some(deadpool_redis::Runtime::Tokio1))
        .expect("redis pool");
    flush_task_views(&redis).await;

    let state = build_state(config).await.expect("build state");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind random port");
    let port = listener.local_addr().expect("local addr").port();

    let server = HttpServer::new(move || App::new().configure(configure(state.clone())))
        .listen(listener)
        .expect("listen")
        .run();
    tokio::spawn(server);

    TestApp {
        base_url: format!("http://127.0.0.1:{port}"),
        client: Client::new(),
        pool,
        redis,
        _guard: guard,
    }
}

async fn flush_task_views(redis: &deadpool_redis::Pool) {
    let mut conn = redis.get().await.expect("redis conn");
    let keys: Vec<String> = conn.keys("tasks:view:*").await.expect("keys");
    if !keys.is_empty() {
        let _: () = conn.del(keys).await.expect("del");
    }
}

impl TestApp {
    pub async fn post(&self, path: &str, body: &Value, token: Option<&str>) -> Response {
        let mut req = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        req.send().await.expect("request")
    }

    pub async fn patch(&self, path: &str, body: &Value, token: Option<&str>) -> Response {
        let mut req = self
            .client
            .patch(format!("{}{}", self.base_url, path))
            .json(body);
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        req.send().await.expect("request")
    }

    pub async fn get(&self, path: &str, token: Option<&str>) -> Response {
        let mut req = self.client.get(format!("{}{}", self.base_url, path));
        if let Some(t) = token {
            req = req.bearer_auth(t);
        }
        req.send().await.expect("request")
    }

    pub async fn seed(&self) -> Value {
        let res = self.post("/seed/users", &json!({}), None).await;
        assert_eq!(res.status(), StatusCode::OK);
        res.json().await.expect("json")
    }

    /// Login only: returns the challenge id and the raw code from the dev inbox.
    pub async fn login(&self, email: &str, password: &str) -> (String, String) {
        let res = self
            .post(
                "/auth/login",
                &json!({ "email": email, "password": password }),
                None,
            )
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = res.json().await.expect("json");
        let challenge = body["login_challenge_id"]
            .as_str()
            .expect("login_challenge_id")
            .to_string();
        let mail = self.get("/dev/email-logs/latest", None).await;
        assert_eq!(mail.status(), StatusCode::OK);
        let mail: Value = mail.json().await.expect("json");
        assert_eq!(mail["to_email"], email);
        let code = mail["verification_code"]
            .as_str()
            .expect("verification_code")
            .to_string();
        (challenge, code)
    }

    pub async fn verify(&self, challenge: &str, code: &str) -> Response {
        self.post(
            "/auth/verify-2fa",
            &json!({ "login_challenge_id": challenge, "code": code }),
            None,
        )
        .await
    }

    /// Full login + 2FA, returning a JWT.
    pub async fn token_for(&self, email: &str, password: &str) -> String {
        let (challenge, code) = self.login(email, password).await;
        let res = self.verify(&challenge, &code).await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Value = res.json().await.expect("json");
        body["access_token"]
            .as_str()
            .expect("access_token")
            .to_string()
    }

    pub async fn create_task(&self, token: &str, title: &str, priority: &str) -> String {
        let res = self
            .post(
                "/tasks",
                &json!({
                    "title": title,
                    "description": format!("{title} description"),
                    "status": "todo",
                    "priority": priority
                }),
                Some(token),
            )
            .await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let body: Value = res.json().await.expect("json");
        body["id"].as_str().expect("id").to_string()
    }

    pub async fn assign(&self, token: &str, ids: &[String], email: &str) -> Response {
        self.post(
            "/tasks/assign",
            &json!({ "task_ids": ids, "assignee_email": email }),
            Some(token),
        )
        .await
    }

    pub async fn view(&self, token: &str) -> Value {
        let res = self.get("/tasks/view-my-tasks", Some(token)).await;
        assert_eq!(res.status(), StatusCode::OK);
        res.json().await.expect("json")
    }

    pub async fn redis_key_exists(&self, key: &str) -> bool {
        let mut conn = self.redis.get().await.expect("redis conn");
        conn.exists(key).await.expect("exists")
    }

    pub async fn user_id(&self, email: &str) -> String {
        let (id,): (uuid::Uuid,) = sqlx::query_as("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&self.pool)
            .await
            .expect("user id");
        id.to_string()
    }

    /// The five James Bond tasks from the assignment, in order.
    pub async fn create_five_tasks(&self, admin: &str) -> Vec<String> {
        let mut ids = Vec::new();
        for (title, priority) in [
            ("Review dossier", "high"),
            ("Check safe house", "medium"),
            ("File report", "low"),
            ("Archive notes", "low"),
            ("Schedule briefing", "medium"),
        ] {
            ids.push(self.create_task(admin, title, priority).await);
        }
        ids
    }
}

pub fn task_view_key(user_id: &str) -> String {
    format!("tasks:view:{user_id}")
}
