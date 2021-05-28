mod consts;
mod da;
mod error;

#[macro_use]
extern crate diesel;

use da::QueueRepository;
use diesel::{Connection, PgConnection};
use futures::Future;
use rand;
use std::{collections::HashMap, env, error::Error, net::Ipv4Addr, str::from_utf8};
use teloxide::{
    net::Download,
    payloads::SendMessageSetters,
    prelude::*,
    types::File,
    utils::command::{BotCommand, ParseError},
};
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
    #[command(
        description = "Swap positions in the queue. Syntax: <b>/swap</b> <u>queue_place1</u> <u>queue_place2</u>",
        parse_with = "split"
    )]
    Swap(i32, i32),
    #[command(
        rename = "queuerand",
        description = "Create a new queue from another queue with shuffling."
    )]
    RandomQueue,
    #[command(
        rename = "queuefile",
        description = "Create a new queue from a file without shuffling."
    )]
    CreateQueueFromFile,
    #[command(
        rename = "insert",
        description = "Add an element to a queue. Syntax: <b>/insert</b> <u>name</u> <u>[@queue_place]</u>. 
    If queue_place isn't provided then inserts to the end of a queue.",
        parse_with = "accept_string_and_number"
    )]
    Insert(String, Option<i32>),
    #[command(
        rename = "remove",
        description = "Remove an element from a queue. Syntax: <b>/remove</b> <u>queue_place</u>"
    )]
    Remove(i32),
}

fn accept_string_and_number(input: String) -> Result<(String, Option<i32>), ParseError> {
    let mut split = input.split("@");
    let string = split.next().map(|x| x.trim());
    let number = split.next().map(|x| x.trim());

    match (string, number) {
        (Some(s), Some(n)) => Ok((
            s.to_string(),
            Some(n.parse::<i32>().map_err(|e| ParseError::Custom(e.into()))?),
        )),
        (Some(s), None) => Ok((s.to_string(), None)),
        _ => Err(ParseError::Custom("Incorrect arguments".into())),
    }
}

async fn run() {
    teloxide::enable_logging!();

    log::info!("Connecting to the database...");
    let conn = establish_connection();

    log::info!("Running migrations...");
    diesel_migrations::run_pending_migrations(&conn)
        .expect("Migrations should be run successfully.");

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
    let chat = repo.get_or_create_chat(chat_id)?;
    let command_handler = CommandHandler {
        repo: repo,
        cx: &cx,
        chat: chat,
    };

    log::info!("Chat: {}; Command: {:?}", chat_id, command);

    match command {
        QueueCommand::Help => {
            cx.answer(QueueCommand::descriptions())
                .parse_mode(teloxide::types::ParseMode::Html)
                .send()
                .await?;
        }
        QueueCommand::Swap(pos1, pos2) => {
            command_handler.swap(pos1, pos2).await?;
        }
        QueueCommand::CreateQueueFromFile => {
            command_handler.queue_from_file().await?;
        }
        QueueCommand::RandomQueue => {
            command_handler.random_queue().await?;
        }
        QueueCommand::Insert(name, index) => {
            command_handler.insert(name, index).await?;
        }
        QueueCommand::Remove(index) => {
            command_handler.remove(index).await?;
        }
    };

    Ok(())
}

fn format_queue(queue: &[da::QueueElementForQueue]) -> String {
    queue
        .iter()
        .map(|x| format!("{}) {}", x.queue_place, x.element_name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn create_queue_based_on(
    repo: &QueueRepository,
    queue_model: &da::Queue,
) -> Result<Vec<da::QueueElementForQueue>, error::Error> {
    let queue = repo.get_elements_for_queue(queue_model)?;
    Ok(shuffled_queue(queue))
}

fn shuffled_queue(mut queue: Vec<da::QueueElementForQueue>) -> Vec<da::QueueElementForQueue> {
    use rand::prelude::*;
    queue.as_mut_slice().shuffle(&mut rand::rngs::OsRng);
    for (i, q) in queue.iter_mut().enumerate() {
        q.queue_place = i as i32 + 1;
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

pub struct CommandHandler<'a> {
    repo: da::QueueRepository,
    cx: &'a UpdateWithCx<Bot, Message>,
    chat: da::Chat,
}

impl CommandHandler<'_> {
    pub async fn random_queue(self) -> error::Result<()> {
        let prev_queue = self.repo.get_previous_queue_for_chat(&self.chat)?;
        match prev_queue {
            Some(prev_queue) => {
                let selected_queue = match self
                    .cx
                    .update
                    .reply_to_message()
                    .map(|Message { id, .. }| {
                        self.repo.queue_exists(da::Queue {
                            id: *id as i64,
                            chat_id: self.chat.id,
                        })
                    })
                    .transpose()?
                    .flatten()
                {
                    Some(q) => q,
                    None => prev_queue,
                };
                let shuffled_queue_elems = create_queue_based_on(&self.repo, &selected_queue)?;

                let str_queue = format_queue(shuffled_queue_elems.as_slice());
                let Message { id: sent_id, .. } = self.cx.answer(str_queue).send().await?;

                let queue = self
                    .repo
                    .create_new_queue(sent_id as i64, selected_queue.chat_id)?;
                self.repo.insert_filled_queue(queue, shuffled_queue_elems)?;
            }
            None => {
                self.cx
                    .answer("You must first call the file variant in this chat elements.")
                    .send()
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn insert(self, name: String, index: Option<i32>) -> error::Result<()> {
        let reply_queue = self
            .cx
            .update
            .reply_to_message()
            .map(|Message { id, .. }| {
                self.repo.queue_exists(da::Queue {
                    id: *id as i64,
                    chat_id: self.chat.id,
                })
            })
            .transpose()?
            .flatten();
        match reply_queue {
            Some(q) => {}
            None => {
                self.cx
                    .answer(
                        "You must reply to a queue created by this bot for this command to work.",
                    )
                    .send()
                    .await?;
            }
        };
        Ok(())
    }

    pub async fn remove(self, index: i32) -> error::Result<()> {
        let reply_queue = self
            .cx
            .update
            .reply_to_message()
            .map(|Message { id, .. }| {
                self.repo.queue_exists(da::Queue {
                    id: *id as i64,
                    chat_id: self.chat.id,
                })
            })
            .transpose()?
            .flatten();
        match reply_queue {
            Some(q) => {}
            None => {
                self.cx
                    .answer(
                        "You must reply to a queue created by this bot for this command to work.",
                    )
                    .send()
                    .await?;
            }
        };
        Ok(())
    }

    pub async fn queue_from_file(self) -> error::Result<()> {
        match self
            .cx
            .update
            .reply_to_message()
            .map(|reply| reply.document())
            .flatten()
        {
            Some(doc) => {
                let File { file_path, .. } = self
                    .cx
                    .requester
                    .get_file(doc.file_id.clone())
                    .send()
                    .await?;
                let mut file_data = Vec::new();
                self.cx
                    .requester
                    .download_file(&file_path, &mut file_data)
                    .await?;

                let str: &str = from_utf8(file_data.as_slice())?;

                let queue_elems = str
                    .lines()
                    .enumerate()
                    .map(|(i, x)| (i + 1, x.trim()))
                    .map(|(i, x)| da::QueueElementForQueue {
                        element_name: x.to_string(),
                        queue_place: i as i32,
                    })
                    .collect::<Vec<_>>();

                let str_queue = format_queue(queue_elems.as_slice());
                let Message { id: sent_id, .. } = self.cx.answer(str_queue).send().await?;

                let queue = self.repo.create_new_queue(sent_id as i64, self.chat.id)?;
                self.repo.insert_filled_queue(queue, queue_elems)?;
            }
            None => {
                self.cx
                    .answer("Please reply to a message with a file.")
                    .send()
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn swap(self, pos1: i32, pos2: i32) -> error::Result<()> {
        let reply_queue = self
            .cx
            .update
            .reply_to_message()
            .map(|Message { id, .. }| {
                self.repo.queue_exists(da::Queue {
                    id: *id as i64,
                    chat_id: self.chat.id,
                })
            })
            .transpose()?
            .flatten();
        match reply_queue {
            Some(reply_queue) => {
                let swap_res = self.repo.swap_positions_for_queue(&reply_queue, pos1, pos2);
                match swap_res {
                    Ok(_) => {
                        let queue: Vec<da::QueueElementForQueue> =
                            self.repo.get_elements_for_queue(&reply_queue)?;
                        let str_queue = format_queue(queue.as_slice());

                        self.cx
                            .requester
                            .edit_message_text(self.chat.id, reply_queue.id as i32, str_queue)
                            .send()
                            .await?;

                        let pos1_name = &queue
                            .iter()
                            .find(|x| x.queue_place == pos1)
                            .unwrap()
                            .element_name;
                        let pos2_name = &queue
                            .iter()
                            .find(|x| x.queue_place == pos2)
                            .unwrap()
                            .element_name;

                        let mut answ = self.cx.answer(format!(
                            "Swapped {} ({}) and {} ({})",
                            pos2_name, pos1, pos1_name, pos2
                        ));
                        answ.reply_to_message_id = Some(self.cx.update.id);
                        answ.send().await?;
                    }
                    Err(da::Error::NonexistentPosition { pos }) => {
                        self.cx
                            .answer(format!("Nonexistent position: {}", pos))
                            .send()
                            .await?;
                    }
                    Err(da::Error::Diesel(e)) => Err(e)?,
                }
            }
            None => {
                self.cx
                    .answer(
                        "You must reply to a queue created by this bot for this command to work.",
                    )
                    .send()
                    .await?;
            }
        }
        Ok(())
    }
}
