mod consts;
mod da;
mod error;

#[macro_use]
extern crate diesel;


use diesel::{Connection, PgConnection};
use futures::Future;
use rand;
use std::{collections::HashMap, env, net::Ipv4Addr, str::from_utf8};
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
        description = "Swap positions in the queue. Syntax: <b>/swap</b> <u>place</u> <u>place</u>",
        parse_with = "split"
    )]
    Swap(i32, i32),
    #[command(
        rename = "queuerand",
        description = "Create a new queue from another queue with shuffling. Syntax: <b>/queuerand</b> <u>[qname]</u>.",
        parse_with = "accept_string_opt"
    )]
    RandomQueue(Option<String>),
    #[command(
        rename = "queuefile",
        description = "Create a new queue from a file without shuffling. Syntax: <b>/queuefile</b> <u>[qname]</u>.",
        parse_with = "accept_string_opt"
    )]
    CreateQueueFromFile(Option<String>),
    #[command(
        rename = "insert",
        description = "Add an element to a queue. Syntax: <b>/insert</b> <u>name</u> <u>[^place]</u>. \
                       If place isn't provided then inserts to the end of a queue.",
        parse_with = "accept_string_and_number"
    )]
    Insert(String, Option<i32>),
    #[command(
        rename = "remove",
        description = "Remove an element from a queue. Syntax: <b>/remove</b> <u>place</u>"
    )]
    Remove(i32),
    #[command(
        rename = "qname",
        description = "Set a new name for the queue. Syntax: <b>/qname</b> <u>new_name</u>"
    )]
    Queuename(String),
}

fn accept_string_and_number(input: String) -> Result<(String, Option<i32>), ParseError> {
    let mut split = input.split("^");
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

fn accept_string_opt(input: String) -> Result<(Option<String>,), ParseError> {
    let trim = input.trim();
    Ok(if trim.len() == 0 {
        (None,)
    } else {
        // optimize allocations
        if trim.len() != input.len() {
            (Some(trim.to_string()),)
        } else {
            (Some(input),)
        }
    })
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

async fn answer(cx: UpdateWithCx<Bot, Message>, command: QueueCommand) -> error::Result<()> {
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

    let res = match command {
        QueueCommand::Help => {
            cx.answer(QueueCommand::descriptions())
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_to_message_id(cx.update.id)
                .send()
                .await?;
            Ok(())
        }
        QueueCommand::Swap(pos1, pos2) => command_handler.swap(pos1, pos2).await,
        QueueCommand::CreateQueueFromFile(name) => command_handler.queue_from_file(name).await,
        QueueCommand::RandomQueue(name) => command_handler.random_queue(name).await,
        QueueCommand::Insert(name, index) => command_handler.insert(name, index).await,
        QueueCommand::Remove(index) => command_handler.remove(index).await,
        QueueCommand::Queuename(qname) => command_handler.set_name(qname).await,
    };

    match res {
        Ok(_) => {}
        Err(error::Error::NoQueueReply) => {
            cx.answer("You must reply to a queue created by this bot for this command to work.")
                .reply_to_message_id(cx.update.id)
                .send()
                .await?;
        }
        Err(error::Error::Diesel(da::Error::NonexistentPosition { pos })) => {
            cx.answer(format!("Nonexistent position: {}", pos))
                .reply_to_message_id(cx.update.id)
                .send()
                .await?;
        }
        Err(e) => return Err(e),
    }

    Ok(())
}

fn format_queue(queue_name: Option<&str>, queue_elems: &[da::QueueElementForQueue]) -> String {
    let elem = queue_elems
        .iter()
        .map(|x| format!("{}) {}", x.queue_place, x.element_name))
        .collect::<Vec<_>>()
        .join("\n");

    match queue_name {
        Some(n) => {
            format!("{}\n{}", n, elem)
        }
        None => elem,
    }
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
    pub async fn random_queue(self, name: Option<String>) -> error::Result<()> {
        let prev_queue = self.repo.get_previous_queue_for_chat(&self.chat)?;
        let prev_queue = match prev_queue {
            Some(prev_queue) => prev_queue,
            None => {
                self.cx
                    .answer("You must first call the file variant in this chat elements.")
                    .reply_to_message_id(self.cx.update.id)
                    .send()
                    .await?;
                return Ok(());
            }
        };

        let selected_queue = match self
            .cx
            .update
            .reply_to_message()
            .map(|Message { id, .. }| {
                self.repo.queue_exists(da::QueueKey {
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

        let queue = self.repo.get_elements_for_queue(&selected_queue.key())?;
        let shuffled_queue_elems = shuffled_queue(queue);

        let str_queue = format_queue(
            name.as_ref().map(|x| x.as_str()),
            shuffled_queue_elems.as_slice(),
        );
        let Message { id: sent_id, .. } = self.cx.answer(str_queue).send().await?;

        let queue = self.repo.create_new_queue(da::Queue {
            id: sent_id as i64,
            chat_id: selected_queue.chat_id,
            qname: name,
        })?;
        self.repo
            .insert_filled_queue(queue.key(), shuffled_queue_elems)?;

        self.cx
            .requester
            .pin_chat_message(self.chat.id, sent_id)
            .send()
            .await?;
        Ok(())
    }

    pub async fn insert(self, name: String, index: Option<i32>) -> error::Result<()> {
        let reply_queue = self.get_reply_to_queue()?;

        self.repo
            .insert_new_elem(&reply_queue.key(), name.clone(), index)?;

        let queue_elem = self.repo.get_elements_for_queue(&reply_queue.key())?;

        let str_queue = format_queue(
            reply_queue.qname.as_ref().map(|x| x.as_str()),
            queue_elem.as_slice(),
        );

        self.cx
            .requester
            .edit_message_text(self.chat.id, reply_queue.id as i32, str_queue)
            .send()
            .await?;

        self.cx
            .answer(format!(
                "Inserted {} at {}",
                name,
                index
                    .map(|x| x.to_string())
                    .unwrap_or("the last position".to_string())
            ))
            .reply_to_message_id(self.cx.update.id)
            .send()
            .await?;

        Ok(())
    }

    pub async fn remove(self, index: i32) -> error::Result<()> {
        let reply_queue = self.get_reply_to_queue()?;

        let removed_name = self.repo.remove_elem(&reply_queue.key(), index)?;

        let queue_elem = self.repo.get_elements_for_queue(&reply_queue.key())?;
        let str_queue = format_queue(
            reply_queue.qname.as_ref().map(|x| x.as_str()),
            queue_elem.as_slice(),
        );

        self.cx
            .requester
            .edit_message_text(self.chat.id, reply_queue.id as i32, str_queue)
            .send()
            .await?;

        self.cx
            .answer(format!("Removed {} from {}", removed_name, index))
            .reply_to_message_id(self.cx.update.id)
            .send()
            .await?;

        Ok(())
    }

    pub async fn queue_from_file(self, name: Option<String>) -> error::Result<()> {
        let doc = match self
            .cx
            .update
            .reply_to_message()
            .map(|reply| reply.document())
            .flatten()
        {
            Some(doc) => doc,
            None => {
                self.cx
                    .answer("Please reply to a message with a file.")
                    .reply_to_message_id(self.cx.update.id)
                    .send()
                    .await?;
                return Ok(());
            }
        };

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

        let str_queue = format_queue(name.as_ref().map(|x| x.as_str()), queue_elems.as_slice());
        let Message { id: sent_id, .. } = self.cx.answer(str_queue).send().await?;

        let queue = self.repo.create_new_queue(da::Queue {
            id: sent_id as i64,
            chat_id: self.chat.id,
            qname: name,
        })?;
        self.repo.insert_filled_queue(queue.key(), queue_elems)?;
        Ok(())
    }

    pub async fn swap(self, pos1: i32, pos2: i32) -> error::Result<()> {
        if pos1 == pos2 {
            self.cx
                .answer("Can't swap position with itself")
                .reply_to_message_id(self.cx.update.id)
                .send()
                .await?;
            return Ok(());
        }

        let reply_queue = self.get_reply_to_queue()?;

        self.repo
            .swap_positions_for_queue(&reply_queue.key(), pos1, pos2)?;

        let queue: Vec<da::QueueElementForQueue> =
            self.repo.get_elements_for_queue(&reply_queue.key())?;
        let str_queue = format_queue(
            reply_queue.qname.as_ref().map(|x| x.as_str()),
            queue.as_slice(),
        );

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

        self.cx
            .answer(format!(
                "Swapped {} ({}) and {} ({})",
                pos2_name, pos1, pos1_name, pos2
            ))
            .reply_to_message_id(self.cx.update.id)
            .send()
            .await?;

        Ok(())
    }

    pub async fn set_name(self, qname: String) -> error::Result<()> {
        let reply_queue = self.get_reply_to_queue()?;
        match reply_queue.qname {
            Some(old_name) if old_name == qname => {
                self.cx
                    .answer("Can't rename to the same name.")
                    .reply_to_message_id(self.cx.update.id)
                    .send()
                    .await?;
                return Ok(());
            }
            _ => {}
        }

        let new_name = self.repo.set_queue_name(&reply_queue.key(), qname)?;

        let queue: Vec<da::QueueElementForQueue> =
            self.repo.get_elements_for_queue(&reply_queue.key())?;
        let str_queue = format_queue(Some(new_name.as_str()), queue.as_slice());

        self.cx
            .requester
            .edit_message_text(self.chat.id, reply_queue.id as i32, str_queue)
            .send()
            .await?;

        self.cx
            .answer(format!(
                "Renamed queue from {} to {}",
                reply_queue.qname.unwrap_or_else(|| "`empty`".to_string()),
                new_name
            ))
            .reply_to_message_id(self.cx.update.id)
            .send()
            .await?;

        Ok(())
    }

    fn get_reply_to_queue(&self) -> error::Result<da::Queue> {
        let reply_queue = self
            .cx
            .update
            .reply_to_message()
            .map(|Message { id, .. }| {
                self.repo.queue_exists(da::QueueKey {
                    id: *id as i64,
                    chat_id: self.chat.id,
                })
            })
            .transpose()?
            .flatten();
        match reply_queue {
            Some(reply_queue) => Ok(reply_queue),
            None => Err(error::Error::NoQueueReply),
        }
    }
}
