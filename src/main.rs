mod db_query;

use std::pin::Pin;
use std::time::Duration;
use uuid::Uuid;
use futures::{Stream, StreamExt};
use juniper::{
    DefaultScalarValue, EmptyMutation, FieldError, graphql_object, graphql_subscription,
    http::GraphQLRequest, RootNode, SubscriptionCoordinator,
};
use juniper_subscriptions::Coordinator;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use tokio::time;

#[derive(Clone)]
pub struct Database {
    pub pool: Pool<Postgres>,
}

impl juniper::Context for Database {}

impl Database {
    fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool
        }
    }
}

pub struct Query;

#[graphql_object(context = Database)]
impl Query {
    fn hello_world() -> &str {
        "Hello World!"
    }
}


struct Task {
    pub id: Uuid,
    pub name: String,
}


#[graphql_object]
impl Task {
    fn id(&self) -> &Uuid { &self.id }
    fn name(&self) -> &str { &self.name }
}

async fn get_task() -> Task {
    Task {
        id: Uuid::new_v4(),
        name: "kek".to_owned()
    }
}


pub struct Subscription;

type CustomStream = Pin<Box<dyn Stream<Item=Result<Task, FieldError>> + Send>>;

static mut START: i32 = 0;

#[graphql_subscription(context = Database)]
impl Subscription {
    async fn hello_world(context: &Database) -> CustomStream {
        // let s: &'static str = context.name.clone();
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

type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

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
    let ctx = Database::new(pool.clone());
    let mut conn = coordinator.subscribe(&req, &ctx).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}

