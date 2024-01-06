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

use serde::Deserialize;

mod server;

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

pub struct Msg{
    pub message: String,
    pub attachment: Option<std::vec::IntoIter<CreateAttachment>>
}

impl Msg {
    pub fn new() -> Msg {
        Msg {
            message: String::new(),
            attachment: None
        }
    }

    pub async fn build_message(&self, msg: &Message) -> Result<Msg, Box<dyn Error>> {
        let txt: String = self.get_content(&msg);
        let image = self.get_image(&msg).await;

        //if txt.is_some() {
        //    let msg: String = txt.clone().unwrap();
        //    let _i = msg.split_ascii_whitespace();
        //}

        let message: Msg = Msg {
            message: txt,
            attachment: image?
        };

        Ok(message)
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

    async fn download_file(&self, img_url: &String, filename: &String) -> Result<CreateAttachment, Box<dyn Error>> {
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
        let mesg: Msg = Msg::new();

        println!("Author: {}\nContent: {}\nAttachments: {:?}\n", msg.author.name, msg.content, msg.attachments);

        let desc: Msg = match mesg.build_message(&msg).await {
            Ok(ok) => ok,
            Err(e) => {
                println!("[Message] Error occured during message parsing: {:?}", e);
                return ();
            }
        };
        let mut author_name = match msg.author.global_name {
            Some(ok) => ok.to_owned(),
            None => msg.author.name.to_owned()
        };
        author_name.push_str(": ");
        author_name.push_str(&desc.message);
        let builder = match desc.attachment {
            Some(ok) => {
                CreateMessage::new()
                    .add_files(ok)
                    .content(author_name)
            },
            None => {
                //println!("[Message] Minor problem: {:?}", e);
                CreateMessage::new()
                    .content(author_name)
            }
        };
        if let Err(why) = msg.channel_id.send_message(&ctx.http, builder).await {
            println!("[Message] Error sending message: {why:?}");
        }

    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("[Startup] INFO {} is online.", ready.user.name);
        server::server::run().await;
    }
}
