use serde::{Deserialize, Serialize};
use sqlx::{migrate::MigrateDatabase,prelude::FromRow, Executor, Sqlite, SqlitePool};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum::{
    extract::State, http::StatusCode, response::{Html, IntoResponse, Response}, routing::get, Form, Router
};
use anyhow::Context;
use askama::Template;

mod error;

use error::Error;

#[derive(Clone)]
struct AppContext {
    pool: SqlitePool,
}

impl AppContext {
    async fn new(url: &str) -> Result<Self, Error> {
        let exist = sqlx::Sqlite::database_exists(url).await?;
        if !exist {
            info!("creating database at {}", url);
            match Sqlite::create_database(url).await {
                Ok(_) => info!("created database at {}", url),
                Err(err) => return Err(err.into()),
            };
        }
        let pool = SqlitePool::connect(url).await?;
        pool.execute(r#"
        CREATE TABLE IF NOT EXISTS record (
        who TEXT NOT NULL,
        message TEXT NOT NULL
        );
        "#).await?;
        Ok(AppContext {pool})
    }
}

impl AppContext {
    async fn insert_entry(&self, entry: Entry) -> Result<(), Error> {
        if let Err(err) = sqlx::query("INSERT INTO record VALUES ($1, $2);")
            .bind(&entry.who)
            .bind(&entry.message).execute(&self.pool).await {
            return Err(err.into())
        }
        Ok(())
    }
    async fn get_entries(&self) -> Result<Vec<Entry>,Error> {
        Ok(sqlx::query_as::<_, Entry>("SELECT who, message FROM record;")
            .fetch_all(&self.pool)
        .await?)
    }
}

#[tokio::test]
async fn test_app_context() {
    let db = AppContext::new(":memory:").await;
    assert!(db.is_ok());
    let pool = db.unwrap();
    let entry = Entry{
        who: "boss".to_string(),
        message: "hi!".to_string(),
    };
    assert!(pool.insert_entry(entry.clone()).await.is_ok());
    let query = pool.get_entries().await;
    assert!(query.is_ok());
    let record = query.unwrap();
    let first = record.first().unwrap();
    assert_eq!(entry, *first);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "raht=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let state = AppContext::new(":memory:").await?;
    let api_router = Router::new()
        .route("/message", get(hello).post(add_entry))
        .with_state(state);
    let router = Router::new()
        .nest("/api", api_router)
        .route("/", get(home));
    let addr = std::net::SocketAddr::from(([0,0,0,0], 3000)).to_string();
    let listener = tokio::net::TcpListener::bind(addr.clone()).await?;
    info!("starting server at {}", addr.clone());
    axum::serve(listener, router.into_make_service())
        .await
        .context("error starting server")?;
    Ok(())
}

#[derive(FromRow, Template, Debug, PartialEq, Clone, Deserialize, Serialize)]
#[template(path="entry.html")]
pub struct Entry {
    who: String,
    message: String,
}

async fn add_entry(
    State(state): State<AppContext>,
    Form(entry): Form<Entry>,
) -> Result<(), Error> {
    match state.insert_entry(entry).await {
        Ok(_) => {
            info!("inserted entry");
            Ok(())
        }
        Err(e) => {
            error!("insert failed: {}", e);
            Err(e)
        }
    }
}

async fn hello(State(state): State<AppContext>) -> Result<Html<String>, Error> {
    match state.get_entries().await {
        Ok(entries) => {
            info!("sending entries...");
            let render: Result<Vec<String>, _> = entries
                .into_iter()
                .map(|entry| entry.render())
                .collect();
            match render {
                Ok(items) => Ok(Html(items.join(""))),
                Err(e) => {
                    error!("fail to render: {}", e);
                    Err(Error::Read)
                }
            }
        }
        Err(e) => {
            error!("get entries failed: {}", e);
            Err(e)
        }
    }
}

async fn home() -> impl IntoResponse {
    info!("home called");
    HtmlTemplate(HomePage {})
}

#[derive(Template,Default)]
#[template(path="home.html")]
struct HomePage; 

struct HtmlTemplate<T>(T);

impl <T:Template> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("fail to render: {}", err)).into_response(),
        }
    }
}
