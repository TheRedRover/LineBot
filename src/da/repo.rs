use diesel::{prelude::*, PgConnection, QueryDsl};

use super::models::{self, Chat, Queue, QueueElement, QueueElementForQueue, QueueKey};
use super::schema;

pub struct QueueRepository {
    conn: PgConnection,
}

impl QueueRepository {
    pub fn from_connection(conn: PgConnection) -> Self {
        QueueRepository { conn }
    }

    pub fn get_chat(&self, chat_id: i64) -> super::error::Result<models::Chat> {
        use schema::chats::dsl::*;
        Ok(chats.filter(id.eq(chat_id)).first::<Chat>(&self.conn)?)
    }

    pub fn create_new_queue(&self, queue: Queue) -> super::error::Result<models::Queue> {
        use schema::queues::dsl::queues;

        Ok(diesel::insert_into(queues)
            .values(queue)
            .get_result(&self.conn)?)
    }

    pub fn insert_filled_queue(
        &self,
        queue: QueueKey,
        queue_elems: Vec<QueueElementForQueue>,
    ) -> super::error::Result<Vec<QueueElement>> {
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

    pub fn get_or_create_chat(&self, chat_id: i64) -> super::error::Result<models::Chat> {
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
        queue: &QueueKey,
    ) -> super::error::Result<Vec<QueueElementForQueue>> {
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

    pub fn get_previous_queue_for_chat(&self, chat: &Chat) -> super::error::Result<Option<Queue>> {
        use schema::queues::dsl::*;

        Ok(queues
            .filter(chat_id.eq(&chat.id))
            .order(id.desc())
            .first::<Queue>(&self.conn)
            .optional()?)
    }

    pub fn queue_exists(&self, queue: QueueKey) -> super::error::Result<Option<Queue>> {
        use schema::queues::dsl::*;

        Ok(
            match queues
                .filter(chat_id.eq(queue.chat_id).and(id.eq(queue.id)))
                .first::<Queue>(&self.conn)
            {
                Ok(queue) => Some(queue),
                Err(diesel::NotFound) => None,
                Err(e) => return Err(e.into()),
            },
        )
    }

    pub fn swap_positions_for_queue(
        &self,
        queue: &QueueKey,
        pos1: i32,
        pos2: i32,
    ) -> Result<(), super::error::Error> {
        use super::error::Error;
        use schema::queue_elements::dsl::*;

        let pos_filter = |pos| {
            queue_elements.filter(
                queue_id
                    .eq(&queue.id)
                    .and(chat_id.eq(&queue.chat_id).and(queue_place.eq(pos))),
            )
        };

        let f_query = |pos| -> Result<_, Error> {
            match pos_filter(pos).first::<QueueElement>(&self.conn) {
                Ok(exists) => Ok(exists),
                Err(diesel::result::Error::NotFound) => {
                    return Err(Error::NonexistentPosition { pos });
                }
                Err(e) => Err(e)?,
            }
        };
        let pos1 = f_query(pos1)?;
        let pos2 = f_query(pos2)?;

        self.conn.transaction::<_, Error, _>(|| {
            diesel::update(pos_filter(pos1.queue_place))
                .set(queue_place.eq(-1))
                .execute(&self.conn)?;

            diesel::update(pos_filter(pos2.queue_place))
                .set(queue_place.eq(pos1.queue_place))
                .execute(&self.conn)?;

            diesel::update(pos_filter(-1))
                .set(queue_place.eq(pos2.queue_place))
                .execute(&self.conn)?;

            Ok(())
        })?;

        Ok(())
    }

    pub fn insert_new_elem(
        &self,
        queue: &QueueKey,
        name: String,
        index: Option<i32>,
    ) -> super::error::Result<()> {
        use super::error::Error;
        use diesel::dsl::*;
        use schema::queue_elements as qe;

        let index: i32 = index
            .map(|x| Ok(Some(x)))
            .unwrap_or_else(|| -> Result<Option<i32>, diesel::result::Error> {
                Ok(qe::table
                    .filter(
                        qe::queue_id
                            .eq(queue.id)
                            .and(qe::chat_id.eq(&queue.chat_id)),
                    )
                    .select(max(qe::queue_place) + 1)
                    .first(&self.conn)?)
            })?
            .ok_or(Error::Wtf(
                "Tried to unwrap but there was no value? How?".to_string(),
            ))?;

        self.conn.transaction(|| -> Result<_, Error> {
            diesel::update(
                qe::table.filter(
                    qe::queue_id
                        .eq(queue.id)
                        .and(qe::chat_id.eq(&queue.chat_id))
                        .and(qe::queue_place.ge(index)),
                ),
            )
            .set(qe::queue_place.eq(qe::queue_place + 1))
            .execute(&self.conn)?;

            diesel::insert_into(qe::table)
                .values(QueueElement {
                    element_name: name,
                    queue_id: queue.id,
                    chat_id: queue.chat_id,
                    queue_place: index,
                })
                .execute(&self.conn)?;

            Ok(())
        })?;

        Ok(())
    }

    pub fn remove_elem(&self, queue: &QueueKey, index: i32) -> super::error::Result<String> {
        use super::error::Error;
        use schema::queue_elements as qe;

        let deleted_name = self.conn.transaction::<_, Error, _>(|| {
            let elem_name = match diesel::delete(
                qe::table.filter(
                    qe::queue_id
                        .eq(queue.id)
                        .and(qe::chat_id.eq(&queue.chat_id))
                        .and(qe::queue_place.eq(index)),
                ),
            )
            .returning(qe::element_name)
            .get_result::<String>(&self.conn)
            {
                Err(diesel::result::Error::NotFound) => {
                    return Err(Error::NonexistentPosition { pos: index });
                }
                v => v,
            }?;

            diesel::update(
                qe::table.filter(
                    qe::queue_id
                        .eq(queue.id)
                        .and(qe::chat_id.eq(&queue.chat_id))
                        .and(qe::queue_place.ge(index)),
                ),
            )
            .set(qe::queue_place.eq(qe::queue_place - 1))
            .execute(&self.conn)?;

            Ok(elem_name)
        })?;

        Ok(deleted_name)
    }

    pub fn set_queue_name(
        &self,
        queue: &QueueKey,
        new_name: String,
    ) -> super::error::Result<String> {
        
        use schema::queues as q;

        let new_name: Option<_> =
            diesel::update(q::table.filter(q::chat_id.eq(queue.chat_id).and(q::id.eq(queue.id))))
                .set(q::qname.eq(new_name))
                .returning(q::qname)
                .get_result(&self.conn)?;
        Ok(new_name.unwrap())
    }
}
