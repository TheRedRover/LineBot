mod consts;

use core::panic;
use futures::lock::Mutex;
use std::env;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::{collections::HashMap, error::Error};
use teloxide::{
    prelude::*,
    types::{File, InputFile},
    utils::command::BotCommand,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{http::Response, Filter};

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
    #[command(description = "Create a new queue from a file.")]
    CreateQueue,
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting the bot...");

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

    let serve = warp::serve(example1).run((Ipv4Addr::UNSPECIFIED, port));

    let bot = Bot::from_env();

    let bot_name = env::var(consts::BOT_NAME)
        .expect(format!("You must provide the {} env variable", consts::BOT_NAME).as_str());

    let repl = teloxide::commands_repl(bot, bot_name, answer);

    tokio::join!(repl, serve);
}

async fn answer(
    cx: UpdateWithCx<Bot, Message>,
    command: QueueCommand,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match command {
        QueueCommand::Help => cx.answer(QueueCommand::descriptions()).send().await?,
        QueueCommand::Swap(_, _) => {
            unimplemented!()
        }
        QueueCommand::CreateQueue => {
            unimplemented!()
        }
    };

    Ok(())
}
