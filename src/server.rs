pub mod server {
    use std::error::Error;
    use std::sync::Arc;

    use serenity::http::Http;
    use serenity::builder::{CreateMessage, CreateAttachment};
    use serenity::model::id::ChannelId;
    use serenity::prelude::RwLock;
    use serenity::prelude::TypeMapKey;

    use axum::{
        routing::post,
        http::StatusCode,
        Json, Router,
    };

    use serde::{Deserialize, Serialize};
    use crate::Msg;
    use crate::Config;

    pub struct ServerWrapper {
        pub server: Server
    }

    impl TypeMapKey for ServerWrapper {
        type Value = Arc<RwLock<ServerWrapper>>;
    }

    #[derive(Debug, Clone)]
    pub struct Server {
        address: String,
        channel_id: u64
    }

    impl Server {
        pub fn build(conf: Config) -> Server {
            let address = conf.server_address;
            Server {
                address,
                //channel_id: 810508141578027028 acds
                channel_id: 1194874734198935633
            }
        }

        pub async fn run(&self) {
            let app = Router::new()
                .route("/post-message", post({
                    let chid = self.channel_id.clone();
                    move |payload| pass_message(payload, chid)
                }));

            println!("Server is running at {}", self.address);

            let listener = tokio::net::TcpListener::bind(self.address.clone()).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        }

        pub async fn send_request(&self, msg: Msg) -> Result<(), Box<dyn Error>> {
            let old_attachments = msg.attachment.iter();
            let mut attachments: Vec<(String, String)> = Vec::with_capacity(msg.attachment.len());

            for val in old_attachments {
                attachments.push((val.filename.clone(), val.url.clone()));
            };

            let new_msg: StagingMsg = StagingMsg {
                text: msg.text,
                author: msg.author,
                attachment: attachments
            };

            let client = reqwest::Client::new();
            let json = serde_json::to_string(&new_msg)?;
            let res = client.post("http:/127.0.0.1:3031/post-message")
                .body(json.clone())
                .header("Content-Type", "application/json")
                .send()
                .await?;

            println!("{}", &json);
            println!("Request response:\n{}\n{}", res.status(), res.text().await?);

            Ok(())
        }
    }

    async fn pass_message(Json(payload): Json<CreateMsg>, channel_id: u64) -> (StatusCode, Json<String>) {
        println!("Delivery:\n{:?}", payload);
        let msg: Msg = Msg {
            author: payload.author,
            text: payload.text,
            attachment: Vec::new()
        };

        let message: String = format!("{}: {}", msg.author, msg.text);
        println!("[LOG] Attachment: {:?}", payload.attachment);
        let file: Option<(String, String)> = if payload.attachment.len() > 0 {
            Some(payload.attachment.first().unwrap().clone()) // Telegram messages with files are
                                                              // usually sent one by one
        } else {
            None
        };
        let attachment: Option<Vec<CreateAttachment>> = match file {
            Some((filename, path)) => match msg.download_file(&path, &filename).await {
                Ok(x) => Some(vec!(x)),
                Err(why) => {return (StatusCode::BAD_REQUEST, Json(why.to_string()));}
            },
            None => None
        };
        if let Err(why) = push_message(message, attachment, channel_id).await {
            return (StatusCode::BAD_REQUEST, Json(why.to_string()));
        };
        let message = String::from("Success");
        (StatusCode::OK, Json(message))
    }

    async fn push_message(text: String, attachment: Option<Vec<CreateAttachment>>, ch_id: u64) -> Result<(), Box<dyn Error>> {
        let conf = Config::build();
        let http = Http::new(&conf?.token);
        let message: CreateMessage = match attachment{
            Some(x) => {
                println!("[INFO] Attachment detected, adding files...");
                CreateMessage::new()
                    .content(text)
                    .add_files(x)
            },
            None => {
                CreateMessage::new()
                    .content(text)
            }
        };

        ChannelId::new(ch_id).send_message(http, message).await?;
        Ok(())
    }

    #[derive(Debug, Deserialize)]
    pub struct CreateMsg {
        text: String,
        author: String,
        attachment: Vec<(String, String)>
    }

    #[derive(Serialize)]
    pub struct StagingMsg {
        text: String,
        author: String,
        attachment: Vec<(String, String)> // (filename, url)
    }
}
