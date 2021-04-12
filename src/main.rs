
use std::pin::Pin;

use futures::{Stream, StreamExt};
use juniper::{
    graphql_object, graphql_subscription, http::GraphQLRequest, DefaultScalarValue, EmptyMutation,
    FieldError, RootNode, SubscriptionCoordinator,
};
use juniper_subscriptions::Coordinator;


use std::time::Duration;
use tokio::time;

struct Weather {
    pub kek: i32
}

#[graphql_object]
impl Weather {
    fn kek(&self) -> &i32 { &self.kek }
}

async fn get_weather(kek_value: i32) -> Weather {
    Weather{
        kek:kek_value
    }
}

const BETWEEN: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct Database;

impl juniper::Context for Database {}

impl Database {
    fn new() -> Self {
        Self {}
    }
}

pub struct Query;

#[graphql_object(context = Database)]
impl Query {
    fn hello_world() -> &str {
        "Hello World!"
    }
}

pub struct Subscription;

type CustomStream = Pin<Box<dyn Stream<Item=Result<Weather, FieldError>> + Send>>;

static mut START: i32 = 0;

#[graphql_subscription(context = Database)]
impl Subscription {
    async fn hello_world() -> CustomStream {
        let stream = futures::stream::unfold((), |state| async  {
            unsafe {
                if START < 10 {
                    START = START + 1;
                    time::delay_for(BETWEEN).await;
                    let weather = get_weather(START).await;
                    Some((Ok(weather), ()))
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
            "query": "subscription { helloWorld {kek} }"
        }"#,
    )
        .unwrap();
    let ctx = Database::new();
    let mut conn = coordinator.subscribe(&req, &ctx).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}

