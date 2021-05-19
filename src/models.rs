use super::schema::*;

#[derive(Queryable, Insertable)]
#[table_name = "chats"]
pub struct Chat {
    pub id: i64,
}

#[derive(Queryable, Insertable)]
#[table_name = "queues"]
pub struct Queues {
    pub id: i64,
    pub chat_id: i64,
}

#[derive(Queryable, Insertable)]
#[table_name = "queue_element"]
pub struct QueueElement {
    pub el_name: String,
    pub queue_id: i64,
    pub chat_id: i64,
    pub queue_place: i32,
}
