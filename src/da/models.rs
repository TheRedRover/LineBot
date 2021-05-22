use super::schema::*;

#[derive(Queryable, Insertable, Clone)]
#[table_name = "chats"]
pub struct Chat {
    pub id: i64,
}

#[derive(Queryable, Insertable, Clone)]
#[table_name = "queues"]
pub struct Queue {
    pub id: i64,
    pub chat_id: i64,
}

#[derive(Queryable, Insertable, Clone)]
#[table_name = "queue_elements"]
pub struct QueueElement {
    pub element_name: String,
    pub queue_id: i64,
    pub chat_id: i64,
    pub queue_place: i32,
}

impl QueueElement {
    pub fn from_parts(queue: Queue, element: QueueElementForQueue) -> Self {
        QueueElement {
            element_name: element.element_name,
            queue_id: queue.id,
            chat_id: queue.chat_id,
            queue_place: element.queue_place,
        }
    }
}

#[derive(Queryable, Clone)]
pub struct QueueElementForQueue {
    pub element_name: String,
    pub queue_place: i32,
}
