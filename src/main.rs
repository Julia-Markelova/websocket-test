use std::pin::Pin;

use futures::{Stream, StreamExt};
use juniper::{
    graphql_object, graphql_subscription, http::GraphQLRequest, DefaultScalarValue, EmptyMutation,
    FieldError, RootNode, SubscriptionCoordinator,
};
use juniper_subscriptions::Coordinator;


use std::time::Duration;
use tokio::time;


#[derive(Clone)]
pub struct Database {
    name: String
}

impl juniper::Context for Database {}

impl Database {
    fn new() -> Self {
        Self {
            name: "hello".to_owned()
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


struct Weather {
    pub kek: i32
}

#[graphql_object]
impl Weather {
    fn kek(&self) -> &i32 { &self.kek }
}

async fn get_weather(kek_value: i32) -> Weather {
    Weather {
        kek: kek_value
    }
}


pub struct Subscription;

type CustomStream = Pin<Box<dyn Stream<Item=Result<Weather, FieldError>> + Send>>;

static mut START: i32 = 0;

#[graphql_subscription(context = Database)]
impl Subscription {
    async fn hello_world(context: &Database) -> CustomStream {
        // https://stackoverflow.com/questions/58700741/is-there-any-way-to-create-a-async-stream-generator-that-yields-the-result-of-re
        let stream = futures::stream::unfold((), |state| async {
            unsafe {
                if START < 5 {
                    // print!("{}", context.name);
                    START = START + 1;
                    time::delay_for(Duration::from_secs(1)).await;
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

