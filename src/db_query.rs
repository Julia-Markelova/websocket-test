use sqlx::{Pool, Postgres};

pub struct Task {
    pub id: uuid::Uuid,
    pub name: String,
}


pub async fn get_task(
    pool: &Pool<Postgres>,
    offset: i32,
) -> Result<Task, sqlx::Error> {
    let (id, name): (uuid::Uuid, String) =
        sqlx::query_as("SELECT id, name \
                            FROM tasks \
                            ORDER BY _created DESC \
                            LIMIT 1 \
                            OFFSET $1 ")
            .bind(offset)
            .fetch_one(pool)
            .await?;

    Ok(Task {
        id,
        name,
    })
}