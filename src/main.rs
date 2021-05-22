mod consts;
mod da;
mod error;

#[macro_use]
extern crate diesel;

use da::{QueueElement, QueueRepository};
use diesel::{prelude::*, Connection, PgConnection, QueryDsl};
use futures::Future;
use rand;
use std::{collections::HashMap, env, error::Error, net::Ipv4Addr, str::from_utf8};
use teloxide::{net::Download, prelude::*, types::File, utils::command::BotCommand};
use warp::{http::Response, Filter};

#[tokio::main]
async fn main() {
    run().await;
}

#[derive(BotCommand, Debug)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum QueueCommand {
    #[command(description = "Obtain help.")]
    Help,
    #[command(description = "Swap positions in the queue.", parse_with = "split")]
    Swap(u32, u32),
    #[command(rename = "queue", description = "Create a new queue.")]
    CreateQueue,
    #[command(rename = "queuefile", description = "Create a new queue from a file.")]
    CreateQueueFromFile,
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
    let repo = da::QueueRepository::from_connection(conn);
    let chat_id = cx.update.chat_id();

    log::info!("Chat: {}; Command: {:?}", chat_id, command);

    match command {
        QueueCommand::Help => {
            cx.answer(QueueCommand::descriptions()).send().await?;
        }
        QueueCommand::Swap(_, _) => {
            unimplemented!()
        }
        QueueCommand::CreateQueueFromFile => {
            let chat = repo.get_or_create_chat(chat_id)?;

            match cx
                .update
                .reply_to_message()
                .map(|reply| reply.document())
                .flatten()
            {
                Some(doc) => {
                    let File { file_path, .. } =
                        cx.requester.get_file(doc.file_id.clone()).send().await?;
                    let mut file_data = Vec::new();
                    cx.requester
                        .download_file(&file_path, &mut file_data)
                        .await?;

                    let str: &str = from_utf8(file_data.as_slice())?;

                    let queue = str
                        .lines()
                        .enumerate()
                        .map(|(i, x)| (i, x.trim()))
                        .map(|(i, x)| da::QueueElementForQueue {
                            element_name: x.to_string(),
                            queue_place: i as i32 + 1,
                        })
                        .collect();

                    let shuffled_queue_elems = shuffled_queue(queue);

                    let str_queue = format_queue(shuffled_queue_elems.as_slice());
                    let Message { id: sent_id, .. } = cx.answer(str_queue).send().await?;

                    let queue = repo.create_new_queue(sent_id as i64, chat.id)?;
                    repo.insert_filled_queue(queue, shuffled_queue_elems)?;
                }
                None => {
                    cx.answer("Please reply to a message with a file.")
                        .send()
                        .await?;
                }
            }
        }
        QueueCommand::CreateQueue => {
            let chat_id = cx.update.chat_id();

            let chat = repo.get_or_create_chat(chat_id)?;

            let prev_queue = repo.get_previous_queue_for_chat(&chat)?;
            match prev_queue {
                Some(prev_queue) => {
                    let selected_queue = match cx
                        .update
                        .reply_to_message()
                        .map(|Message { id, .. }| {
                            repo.queue_exists(da::Queue {
                                id: *id as i64,
                                chat_id: chat.id,
                            })
                        })
                        .transpose()?
                        .flatten()
                    {
                        Some(q) => q,
                        None => prev_queue,
                    };
                    let shuffled_queue_elems = create_queue_based_on(&repo, &selected_queue)?;

                    let str_queue = format_queue(shuffled_queue_elems.as_slice());
                    let Message { id: sent_id, .. } = cx.answer(str_queue).send().await?;

                    let queue = repo.create_new_queue(sent_id as i64, selected_queue.chat_id)?;
                    repo.insert_filled_queue(queue, shuffled_queue_elems)?;
                }
                None => {
                    cx.answer("You must first call the file variant in this chat elements.")
                        .send()
                        .await?;
                }
            }
        }
    };

    Ok(())
}

fn format_queue(queue: &[da::QueueElementForQueue]) -> String {
    queue
        .iter()
        .map(|x| format!("{}) {}", x.queue_place + 1, x.element_name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn create_queue_based_on(
    repo: &QueueRepository,
    queue_model: &da::Queue,
) -> Result<Vec<da::QueueElementForQueue>, error::Error> {
    let mut queue = repo.get_elements_for_queue(queue_model)?;
    Ok(shuffled_queue(queue))
}

fn shuffled_queue(mut queue: Vec<da::QueueElementForQueue>) -> Vec<da::QueueElementForQueue> {
    use rand::prelude::*;
    queue.as_mut_slice().shuffle(&mut rand::rngs::OsRng);
    for (i, q) in queue.iter_mut().enumerate() {
        q.queue_place = i as i32;
    }
    queue
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
