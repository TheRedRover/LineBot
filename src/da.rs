use core::panic;
use diesel::prelude::*;
use diesel::{Connection, PgConnection, QueryDsl};
use futures::{lock::Mutex, Future};
use models::{Chat, QueueElement};
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::{collections::HashMap, error::Error};
use std::{env, str::from_utf8};
use teloxide::{
    net::Download,
    prelude::*,
    types::{File, InputFile},
    utils::command::BotCommand,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{http::Response, Filter};

use crate::schema;
use crate::models;
use crate::models::Queues;

pub fn get_chat(conn: &PgConnection, chat_id: i64) -> Result<models::Chat, diesel::result::Error> {
    use schema::chats::dsl::*;
    chats.filter(id.eq(chat_id)).first::<Chat>(conn)
}

pub fn create_new_queue(
    conn: &PgConnection,
    id: i64,
    chat_id: i64,
) -> Result<models::Queues, diesel::result::Error> {
    use schema::queues::dsl::queues;

    Ok(diesel::insert_into(queues)
        .values(Queues {
            id: id,
            chat_id: chat_id,
        })
        .get_result(conn)?)
}

pub fn populate_queue(
    conn: &PgConnection,
    queue_elems: Vec<QueueElement>,
) -> Result<Vec<QueueElement>, diesel::result::Error> {
    use schema::queue_element::dsl::*;

    Ok(diesel::insert_into(queue_element)
        .values(queue_elems)
        .get_results(conn)?)
}

pub fn get_or_create_chat(
    conn: &PgConnection,
    chat_id: i64,
) -> Result<models::Chat, diesel::result::Error> {
    use schema::chats::dsl::*;
    Ok(match get_chat(conn, chat_id) {
        Ok(c) => c,
        Err(_) => diesel::insert_into(chats)
            .values(Chat { id: chat_id })
            .get_result(conn)?,
    })
}