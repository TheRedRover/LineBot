use crate::schema::*;

#[derive(Queryable, Insertable, Identifiable)]
#[table_name = "chats"]
pub struct Chat {
    pub id: i64,
}

#[derive(Queryable, Insertable, Identifiable, Associations)]
#[table_name = "queues"]
#[primary_key(id, chat_id)]
pub struct Queues {
    pub id: i64,
    pub chat_id: i64,
}

#[derive(Queryable, Insertable, Identifiable)]
#[table_name = "queue_element"]
#[primary_key(el_name, queue_id, chat_id)]
pub struct QueueElement {
    pub el_name: String,
    pub queue_id: i64,
    pub chat_id: i64,
    pub queue_place: i32,
}
