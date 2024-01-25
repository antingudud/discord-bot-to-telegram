pub mod server {
    use std::error::Error;
    use std::sync::Arc;
    use std::collections::HashMap;

    use serenity::http::Http;
    use serenity::builder::{CreateMessage, CreateAttachment, CreateForumPost};
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

    #[derive(Clone)]
    pub struct Server {
        address: String,
        //channel_id: u64,
        forum_id: u64,
        global_data: Arc<tokio::sync::RwLock<crate::DataWrapper>>
    }

    impl Server {
        pub fn build(conf: Config, global_data: Arc<tokio::sync::RwLock<crate::DataWrapper>>) -> Server {
            let address = conf.server_address;
            Server {
                address,
                //channel_id: 810508141578027028 acds
                //channel_id: 1194874734198935633,
                forum_id: 1195650169216184370,
                global_data
            }
        }

        pub async fn run(&self) {
            let app = Router::new()
                .route("/post-message", post({
                    let server = self.clone();

                    move |payload| pass_message(payload, server)
                }))
                .route("/init", post({
                    let server = self.clone();

                    move |payload| ticket_init(payload, server)
                }))
                .route("/close", post({
                    let server = self.clone();

                    move |payload| ticket_close(payload, server)
                }));

            println!("Server is running at {}", self.address);

            let listener = tokio::net::TcpListener::bind(self.address.clone()).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        }

        pub async fn send_request(&self, msg: Msg, tele_id: i64) -> Result<(), Box<dyn Error>> {
            let old_attachments = msg.attachment.iter();
            let mut attachments: Vec<(String, String)> = Vec::with_capacity(msg.attachment.len());

            for val in old_attachments {
                attachments.push((val.filename.clone(), val.url.clone()));
            };

            let new_msg: PayloadMsg = PayloadMsg {
                tele_id,
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

    // Functions below should get it's own discord module
    async fn ticket_close(Json(payload): Json<PayloadClose>, server: Server) -> (StatusCode, Json<ResponseMsg>) {
        println!("Closing ticket...");

        let fid: Option<u64> = {
            let data_read = server.global_data.read().await;
            let hm_id_tg = data_read.tele_id.clone();
            match hm_id_tg {
                Some(hm) => {
                    match hm.get(&Some(payload.id)) {
                        Some(fid) => {
                            if fid.is_none() {None}
                            else {Some(fid.unwrap())}
                        },
                        None => None
                    }
                },
                None => None
            }
        };

        if fid.is_none() {
            let res = ResponseMsg {
                status: "fail".to_string(),
                code: 2,
                message: "forum not found".to_string()
            };
            return (StatusCode::NOT_FOUND, Json(res))
        }

        let ctx = {
            let data_read = server.global_data.read().await;
            let context = data_read.context.clone();
            context.unwrap()
        };

        let channel = ChannelId::new(fid.unwrap());
        let editthread = serenity::builder::EditThread::new().locked(true);
        match channel.edit_thread(ctx, editthread).await {
            Ok(_) => {},
            Err(why) => {
                let res = ResponseMsg {
                    status: "fail".to_string(),
                    code: 1,
                    message: why.to_string()
                };
                return (StatusCode::OK, Json(res));
            }
        };

        {
            let mut data_write = server.global_data.write().await;
            let tg = data_write.tele_id.clone();
            let dc = data_write.disc_id.clone();
            let mut hm_id_tg = tg.unwrap();
            let mut hm_id_dc = dc.unwrap();
            match hm_id_tg.remove(&Some(payload.id)) {
                Some(w) => {println!("[LOG] hm_id_tg item deleted. Check: {:?}", w)},
                None => {println!("[ERROR] at hm_id_tg removal")}
            };
            match hm_id_dc.remove(&Some(fid.unwrap())) {
                Some(w) => {println!("[LOG] hm_id_dc item deleted. Check: {:?}", w)},
                None => {println!("[ERROR] at hm_id_dc removal")}
            };

            data_write.tele_id = Some(hm_id_tg.clone());
            data_write.disc_id = Some(hm_id_dc.clone());
        }

        {
            let data_read = server.global_data.read().await;
            let hm_id_dc = data_read.disc_id.clone();
            println!("[LOG] Current channelid: {} discord hash map: {:?}", fid.unwrap(), hm_id_dc.unwrap());
        }

        let res = ResponseMsg {
            status: "success".to_string(),
            code: 0,
            message: "ticket closed".to_string()
        };
        (StatusCode::OK, Json(res))
    }
    async fn ticket_init(Json(payload): Json<PayloadInit>, server: Server) -> (StatusCode, Json<ResponseForum>) {
        println!("Creating forum post...");
    
        let chid: ChannelId = ChannelId::new(server.forum_id.clone());
        let ctx = {
            let data_read = server.global_data.read().await;
            data_read.context.clone().unwrap()
        };
        let cfp = CreateForumPost::new(payload.title, CreateMessage::new().content("_ _".to_string()));

        let fid: u64;

        // forum existing check
        {
            let data_read = server.global_data.read().await;
            let hm_id_tg = data_read.tele_id.clone();
            if hm_id_tg.is_some() {
                if hm_id_tg.unwrap().get(&Some(payload.id)).is_some() {
                    let res = ResponseForum {
                        status: "fail".to_string(),
                        code: 2,
                        message: "forum exists".to_string(),
                        id: 0
                    };
                    return (StatusCode::BAD_REQUEST, Json(res));
                }
            }
        }

        match chid.create_forum_post(ctx, cfp).await {
            Ok(v) => {
                fid = v.id.get();
            },
            Err(why) => {
                println!("[ERROR] at init: {:?}", why);
                let res = ResponseForum {
                    status: "fail".to_string(),
                    code: 1,
                    message: why.to_string(),
                    id: 0
                };
                return (StatusCode::BAD_REQUEST, Json(res));
            }
        }

        {
            let mut un = server.global_data.write().await;
            // Add discord HashMap
            let disc = un.disc_id.clone();
            match disc {
                Some(mut x) => {
                    x.insert(Some(fid.clone()), Some(payload.id));
                    un.disc_id = Some(x);
                },
                None => {
                    let mut hm_id_dc: HashMap<Option<u64>, Option<i64>> = HashMap::new();
                    hm_id_dc.insert(Some(fid.clone()), Some(payload.id));
                    un.disc_id = Some(hm_id_dc);
                }
            }
            // Add telegram HashMap
            let tele = un.tele_id.clone();
            match tele {
                Some(mut x) => {
                    x.insert(Some(payload.id), Some(fid.clone()));
                    un.tele_id = Some(x);
                },
                None => {
                    let mut hm_id_tg: HashMap<Option<i64>, Option<u64>> = HashMap::new();
                    hm_id_tg.insert(Some(payload.id), Some(fid.clone()));
                    un.tele_id = Some(hm_id_tg);
                }
            }
        }

        let res = ResponseForum {
            status: "success".to_string(),
            code: 0,
            message: "forum post created".to_string(),
            id: fid
        };
        (StatusCode::OK, Json(res))
    }  

    async fn pass_message(Json(payload): Json<PayloadMsg>, server: Server) -> (StatusCode, Json<ResponseMsg>) {
        println!("Delivery:\n{:?}", payload);
        let msg: Msg = Msg {
            author: payload.author,
            text: payload.text,
            attachment: Vec::new()
        };

        let hm_id_tg: Option<HashMap<Option<i64>, Option<u64>>> = {
            let dr = server.global_data.read().await;
            dr.tele_id.clone()
        };
        println!("[LOG] at pass_message: hm_id_tg {:?}", hm_id_tg.clone());

        let forum_id: Option<u64> = match hm_id_tg {
            Some(v) => {
                match v.get(&Some(payload.tele_id)) {
                    Some(x) => {
                        if x.is_none() { None }
                        else { Some(x.unwrap()) }
                    },
                    None => None
                }
            },
            None => None
        };

        if forum_id.is_none() {
            let x: ResponseMsg = ResponseMsg {
                status: "fail".to_string(),
                code: 2,
                message: "no forum".to_string()
            };
            return (StatusCode::BAD_REQUEST, Json(x));
        }

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
                Err(why) => {
                    let x: ResponseMsg = ResponseMsg {
                        status: "fail".to_string(),
                        code: 1,
                        message: why.to_string()
                    };
                    return (StatusCode::BAD_REQUEST, Json(x));
                }
            },
            None => None
        };
        if let Err(why) = push_message(message, attachment, forum_id.unwrap()).await {
            let x: ResponseMsg = ResponseMsg {
                status: "fail".to_string(),
                code: 1,
                message: why.to_string()
            };
            return (StatusCode::BAD_REQUEST, Json(x));
        };
        let succ: ResponseMsg = ResponseMsg {
            status: "success".to_string(),
            code: 0,
            message: "Success".to_string()
        };
        (StatusCode::OK, Json(succ))
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

    #[derive(Deserialize)]
    pub struct PayloadInit {
        id: i64,
        title: String
    }
    #[derive(Deserialize)]
    pub struct PayloadClose {
        id: i64
    }
    #[derive(Serialize)]
    pub struct ResponseForum {
        status: String,
        code: u8,
        message: String,
        id: u64 // forum post id
    }
    #[derive(Serialize)] 
    pub struct ResponseMsg {
        status: String,
        code: u32, // 0: success. 1: system fail. 2: logic fail
        message: String
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct PayloadMsg {
        tele_id: i64,
        text: String,
        author: String,
        attachment: Vec<(String, String)> // (filename, url)
    }
}
