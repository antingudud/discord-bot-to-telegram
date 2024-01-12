use std::fs;
use std::error::Error;
use std::env;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::channel::Attachment;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::builder::CreateMessage;
use serenity::builder::CreateAttachment;

use serde::{Deserialize, Serialize};

pub mod server;

#[derive(Debug)]
#[derive(Deserialize)]
pub struct Config {
    pub token: String,
    pub server_address: String
}

impl Config {
    pub fn build() -> std::result::Result<Config, Box<dyn Error>> {
        let config_path: &str = match env::var("TEST") {
            Ok(_) => "config.example.json",
            Err(_) => "config.json"
        };

        let config_file = fs::read_to_string(config_path)?;
        let conf: Config = serde_json::from_str(&config_file)?;

        Ok(conf)
    }
}

#[derive(Serialize)]
pub struct Msg{
    pub author: String,
    pub text: String,
    pub attachment: Vec<Attachment>
}

impl Msg {
    pub fn new() -> Msg { // remove this later
        Msg {
            author: String::new(),
            text: String::new(),
            attachment: Vec::new()
        }
    }

    pub fn build(msg: &Message) -> Msg {
        Msg {
            author: match &msg.author.global_name {
                Some(x) => x.to_string(),
                None => msg.author.name.clone()
            },
            text: msg.content.clone(),
            attachment: msg.attachments.clone()
        }
    }

    pub fn get_content(&self, msg: &Message) -> String {
        let content = msg.content.clone();
        let desc: String = if content.len() < 1 {"_ _".to_string()} else {content};

        desc
    }

    pub async fn get_image(&self, msg: &Message) -> Result<Option<std::vec::IntoIter<CreateAttachment>>, Box<dyn Error>> {
        let urls: Option<&Vec<Attachment>> = if msg.attachments.len() == 0 {
            //return None;
            None
        } else {
            Some(&msg.attachments)
        };
        let result = match urls {
            None => None,
            Some(x) => {
                let v = x.iter();
                let mut a: Vec<CreateAttachment> = Vec::new();

                for val in v {
                    a.push(self.download_file(&val.url, &val.filename).await?);
                }

                let d = a.into_iter();
                Some(d)
            }
        };

        Ok(result)
    }

    pub async fn download_file(&self, img_url: &String, filename: &String) -> Result<CreateAttachment, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let res = client.get(img_url)
            .send()
            .await?
            .bytes()
            .await?;

        let byte: Vec<u8> = Vec::from(res);
        let attachment = CreateAttachment::bytes(byte, filename);

        return Ok(attachment);
    }
}


pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let self_id: u64 = 1192895878009192508;
        if msg.author.id.get() == self_id {
            return ();
        }
        if msg.content.starts_with("|") {
            return ();
        }
        let mesg: Msg = Msg::build(&msg);
        let lock = {
            let data_read = ctx.data.read().await;
            data_read.get::<server::server::ServerWrapper>().unwrap().clone()
        };

        {
            let server_wrapper = lock.write().await;
            if let Err(why) = server_wrapper.server.send_request(mesg).await {
                println!("[ERROR] At message handler: {:?}", why);
            };
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("[Startup] INFO {} is online.", ready.user.name);
    }
}
