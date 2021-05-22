use diesel::{prelude::*, PgConnection, QueryDsl};

use super::models::{self, Chat, Queue, QueueElement, QueueElementForQueue};
use super::schema;
use teloxide::{
    net::Download,
    prelude::*,
    types::{File, InputFile},
    utils::command::BotCommand,
};

pub struct QueueRepository {
    conn: PgConnection,
}

impl QueueRepository {
    pub fn from_connection(conn: PgConnection) -> Self {
        QueueRepository { conn }
    }

    pub fn get_chat(&self, chat_id: i64) -> Result<models::Chat, diesel::result::Error> {
        use schema::chats::dsl::*;
        chats.filter(id.eq(chat_id)).first::<Chat>(&self.conn)
    }

    pub fn create_new_queue(
        &self,
        id: i64,
        chat_id: i64,
    ) -> Result<models::Queue, diesel::result::Error> {
        use schema::queues::dsl::queues;

        Ok(diesel::insert_into(queues)
            .values(Queue { id, chat_id })
            .get_result(&self.conn)?)
    }

    pub fn insert_filled_queue(
        &self,
        queue: Queue,
        queue_elems: Vec<QueueElementForQueue>,
    ) -> Result<Vec<QueueElement>, diesel::result::Error> {
        use schema::queue_elements::dsl::*;

        Ok(diesel::insert_into(queue_elements)
            .values(
                queue_elems
                    .into_iter()
                    .map(|x| QueueElement::from_parts(queue.clone(), x))
                    .collect::<Vec<QueueElement>>(),
            )
            .get_results(&self.conn)?)
    }

    pub fn get_or_create_chat(&self, chat_id: i64) -> Result<models::Chat, diesel::result::Error> {
        use schema::chats::dsl::*;
        Ok(match self.get_chat(chat_id) {
            Ok(c) => c,
            Err(_) => diesel::insert_into(chats)
                .values(Chat { id: chat_id })
                .get_result(&self.conn)?,
        })
    }

    pub fn get_elements_for_queue(
        &self,
        queue: &Queue,
    ) -> Result<Vec<QueueElementForQueue>, diesel::result::Error> {
        use queue_elements as qe;
        use schema::*;

        Ok(queues::table
            .inner_join(
                queue_elements::table.on(queues::id
                    .eq(queue_elements::queue_id)
                    .and(queues::chat_id.eq(queue_elements::chat_id))),
            )
            .filter(
                queues::id
                    .eq(&queue.id)
                    .and(queues::chat_id.eq(&queue.chat_id)),
            )
            .order(qe::queue_place)
            .select((qe::element_name, qe::queue_place))
            .load::<QueueElementForQueue>(&self.conn)?)
    }

    pub fn get_previous_queue_for_chat(
        &self,
        chat: &Chat,
    ) -> Result<Option<Queue>, diesel::result::Error> {
        use schema::queues::dsl::*;

        Ok(
            match queues
                .filter(chat_id.eq(&chat.id))
                .order(id.desc())
                .first::<Queue>(&self.conn)
            {
                Ok(queue) => Some(queue),
                Err(diesel::NotFound) => None,
                Err(e) => return Err(e),
            },
        )
    }

    pub fn queue_exists(&self, queue: Queue) -> Result<Option<Queue>, diesel::result::Error> {
        use schema::queues::dsl::*;

        Ok(
            match queues
                .filter(chat_id.eq(queue.chat_id).and(id.eq(queue.id)))
                .first::<Queue>(&self.conn)
            {
                Ok(queue) => Some(queue),
                Err(diesel::NotFound) => None,
                Err(e) => return Err(e),
            },
        )
    }
}
