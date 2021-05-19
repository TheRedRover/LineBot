mod consts;
mod models;
mod schema;

#[macro_use]
extern crate diesel;

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

use crate::models::Queues;

#[tokio::main]
async fn main() {
    run().await;
}

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum QueueCommand {
    #[command(description = "Obtain help.")]
    Help,
    #[command(description = "Swap positions in the queue.", parse_with = "split")]
    Swap(u32, u32),
    #[command(rename = "queue", description = "Create a new queue.")]
    CreateQueue,
    #[command(rename = "addelem", description = "Create a new queue from a file.")]
    AddElementsToQueue,
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting the bot...");

    let repl = create_bot();
    let serve = create_http_server();

    tokio::join!(repl, serve);
}

async fn answer(
    cx: UpdateWithCx<Bot, Message>,
    command: QueueCommand,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let conn = establish_connection();
    let chat_id = cx.update.chat_id();

    match command {
        QueueCommand::Help => cx.answer(QueueCommand::descriptions()).send().await?,
        QueueCommand::Swap(_, _) => {
            unimplemented!()
        }
        QueueCommand::AddElementsToQueue => {
            use models::Chat;

            let chat = get_or_create_chat(&conn, chat_id)?;
            let queue = create_new_queue(&conn, cx.update.id as i64, chat.id)?;

            match cx.update.document() {
                Some(doc) => {
                    let File { file_path, .. } =
                        cx.requester.get_file(doc.file_id.clone()).send().await?;
                    let mut file_data = Vec::new();
                    cx.requester
                        .download_file(&file_path, &mut file_data)
                        .await?;

                    let str: &str = from_utf8(file_data.as_slice())?;

                    let elems = str
                        .lines()
                        .enumerate()
                        .map(|(i, x)| (i, x.trim()))
                        .map(|(i, x)| QueueElement {
                            chat_id: chat.id,
                            el_name: x.to_string(),
                            queue_id: queue.id,
                            queue_place: i as i32 + 1,
                        })
                        .collect();

                    populate_queue(&conn, elems)?;
                }
                None => {
                    cx.answer("Please provide a file.").send().await?;
                }
            }

            unimplemented!()
        }
        QueueCommand::CreateQueue => {
            let chat_id = cx.update.chat_id();

            match get_chat(&conn,chat_id){
                Ok(chat) => {

                }
                Err(_) => {
                    cx.answer("You must first create add elements.").send().await?;
                }
            }

            unimplemented!()
        }
    };

    Ok(())
}

fn get_or_create_chat(
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

fn get_chat(conn: &PgConnection, chat_id: i64) -> Result<models::Chat, diesel::result::Error> {
    use schema::chats::dsl::*;
    chats.filter(id.eq(chat_id)).first::<Chat>(conn)
}

fn create_new_queue(
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

fn populate_queue(
    conn: &PgConnection,
    queue_elems: Vec<QueueElement>,
) -> Result<Vec<QueueElement>, diesel::result::Error> {
    use schema::queue_element::dsl::*;

    Ok(diesel::insert_into(queue_element)
        .values(queue_elems)
        .get_results(conn)?)
}

fn create_bot() -> impl Future {
    let bot = Bot::from_env();

    let bot_name = env::var(consts::BOT_NAME)
        .expect(format!("You must provide the {} env variable", consts::BOT_NAME).as_str());

    teloxide::commands_repl(bot, bot_name, answer)
}

fn establish_connection() -> PgConnection {
    let database_url = env::var(consts::DATABASE_URL)
        .expect(format!("{} must be set", consts::DATABASE_URL).as_str());
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

fn create_http_server() -> impl Future {
    let example1 = warp::get()
    .and(warp::query::<HashMap<String, String>>())
    .map(|p: HashMap<String, String>| match p.get("name") {
        Some(name) => Response::builder().body(format!("Hello, {}. This HTTP triggered function executed successfully.", name)),
        None => Response::builder().body(String::from("This HTTP triggered function executed successfully. Pass a name in the query string for a personalized response.")),
    });
    let port: u16 = match env::var(consts::HANDLER_PORT) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    warp::serve(example1).run((Ipv4Addr::UNSPECIFIED, port))
}
