use std::env;

use chrono::{TimeZone, Timelike, Utc};
use chrono_tz::Europe::Athens;
use teloxide::{prelude::*, types::InputFile};
use tokio_stream::wrappers::UnboundedReceiverStream;

use chrono::DateTime;
use chrono_tz::Tz;
use futures::lock::Mutex;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use warp::{http::Response, Filter};

#[tokio::main]
async fn main() {
    run().await;
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

    let port_key = "BOT_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    let serve = warp::serve(example1).run((Ipv4Addr::UNSPECIFIED, port));

    let bot = Bot::from_env();

    let map = Arc::new(Mutex::new(HashMap::new()));

    let dispatch = async move {
        let x = Dispatcher::new(bot).messages_handler(|rx| handle_messages(rx, map));
        x.dispatch().await
    };

    tokio::join!(dispatch, serve);
}

async fn handle_messages(
    rx: DispatcherHandlerRx<Bot, Message>,
    chat_last_self_msg: Arc<Mutex<HashMap<i64, DateTime<Tz>>>>,
) {
    let ref_chat_map = &chat_last_self_msg;
    UnboundedReceiverStream::new(rx)
        .for_each_concurrent(None, |msg| async move {
            log::info!("msg: {:?}", msg.update);
            match &msg.update.kind {
                teloxide::types::MessageKind::Common(_) => {
                    let time = Utc::now().naive_utc();
                    let time = Athens.from_utc_datetime(&time);
                    let hour = time.hour();
                    let late_at_night = hour <= 6;
                    log::info!(
                        "time: {}; hour: {}; late_at_night?: {}",
                        time,
                        hour,
                        late_at_night
                    );

                    let debug_respond_always = env::var("TG_BOT_RESPOND_ALWAYS_DEBUG");
                    log::info!("debug?: {}", debug_respond_always.is_ok());

                    let debug_ignore_late_at_night = env::var("TG_BOT_IGNORE_NIGHT_DEBUG");
                    log::info!("ignore night?: {}", debug_ignore_late_at_night.is_ok());

                    if debug_ignore_late_at_night.is_ok() || late_at_night {
                        let chat_map = ref_chat_map.clone();
                        let mut chat_map = chat_map.lock().await;

                        if debug_respond_always.is_ok()
                            || chat_map
                                .get(&msg.update.chat_id())
                                .map(|prev_time| {
                                    let mins = (time - *prev_time).num_minutes();
                                    log::info!(
                                        "minutes passed from the time of the last bots message: {}",
                                        mins
                                    );
                                    mins > 60
                                })
                                .unwrap_or(true)
                        {
                            log::info!("sending a message...");
                            let mut resp =
                                msg.answer_photo(InputFile::File("resources/img.jpg".into()));
                            resp.reply_to_message_id = Some(msg.update.id);
                            resp.send().await.log_on_error().await;

                            log::info!("message sent.");

                            chat_map.insert(msg.update.chat_id(), time);
                        }
                    }
                }
                _ => (),
            }
        })
        .await;
}
