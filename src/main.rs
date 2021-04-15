use std::pin::Pin;
use std::time::Duration;

use futures::{Stream, StreamExt};
use juniper::{DefaultScalarValue, EmptyMutation, FieldError, graphql_object, graphql_subscription, http::GraphQLRequest, RootNode, SubscriptionCoordinator, Value};
use juniper::GraphQLEnum;
use juniper_subscriptions::Coordinator;
use serde::Deserialize;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use tokio::time;
use uuid::Uuid;

mod db_query;

#[derive(Clone)]
pub struct WebSocketContext {
    pub pool: Pool<Postgres>,
    pub traces_dir: String,
}

impl juniper::Context for WebSocketContext {}

impl WebSocketContext {
    fn new(pool: Pool<Postgres>, traces_dir: String) -> Self {
        Self {
            pool,
            traces_dir
        }
    }
}

pub struct Query;

#[graphql_object(context = WebSocketContext)]
impl Query {
    fn hello_world() -> &str {
        "Hello World!"
    }
}


#[derive(GraphQLEnum, Deserialize)]
pub enum TaskStatus {
    DRAFT,
    SENT,
    PREPARING,
    SOLVING,
    SOLVED,
    FAILED,
}


struct Task {
    pub id: Uuid,
    pub name: String,
    pub status: TaskStatus,
}


#[graphql_object]
impl Task {
    fn id(&self) -> &Uuid { &self.id }
    fn name(&self) -> &str { &self.name }
    fn status(&self) -> &TaskStatus {&self.status}
}

async fn get_task(traces_dir: &str, index: i32) -> Task {
    let mut status;
    if index == 1 {
        status = TaskStatus::DRAFT
    }
    if index == 2 {
        status = TaskStatus::SENT
    }
    if index == 3 {
        status = TaskStatus::SOLVING
    }
    else {
        status = TaskStatus::SOLVED
    }
    Task {
        id: Uuid::new_v4(),
        name: "kek".to_owned(),
        status: status,
    }
}


pub struct Subscription;

type CustomStream = Pin<Box<dyn Stream<Item=Result<Task, FieldError>> + Send>>;

static mut START: i32 = 0;

#[graphql_subscription(context = WebSocketContext)]
impl Subscription {
    async fn hello_world(context: &WebSocketContext, task_id: Uuid) -> CustomStream {
        // https://stackoverflow.com/questions/58700741/is-there-any-way-to-create-a-async-stream-generator-that-yields-the-result-of-re
        let stream = futures::stream::unfold((), |state| async {
            unsafe {
                if START < 5 {
                    // print!("{}", s);
                    START = START + 1;
                    time::delay_for(Duration::from_secs(1)).await;
                    let task = get_task().await;
                    Some((Ok(task), ()))
                } else {
                    None
                }
            }
        });
        Box::pin(stream)
    }
}

type Schema = RootNode<'static, Query, EmptyMutation<WebSocketContext>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new(), Subscription {})
}

#[tokio::main]
async fn main() {
    let schema = schema();
    let coordinator = Coordinator::new(schema);
    let req: GraphQLRequest<DefaultScalarValue> = serde_json::from_str(
        r#"{
            "query": "subscription { helloWorld {id name} }"
        }"#,
    )
        .unwrap();
    let database_url = String::from("postgres://postgres:1234@localhost:55436/plan_design");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&*database_url).await.unwrap();
    let storage_dir: String = String::from("some dir");
    let ctx = WebSocketContext::new(pool.clone(), storage_dir.clone());
    let mut conn = coordinator.subscribe(&req, &ctx).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}



// https://stackoverflow.com/questions/65101589/how-does-one-use-sqlx-with-juniper-subscriptions-in-rust