pub mod server {
    use std::error::Error;

    use serenity::http::Http;
    use serenity::builder::CreateMessage;
    use serenity::model::id::ChannelId;

    use axum::{
        routing::post,
        http::StatusCode,
        Json, Router,
    };

    use serde::Deserialize;
    use crate::Msg;
    use crate::Config;

    #[derive(Debug)]
    pub struct Server {
        address: String,
        channel_id: u64
    }

    impl Server {
        pub fn build() -> Server {
            let conf = Config::build();
            let address = conf.unwrap().server_address;
            Server {
                address,
                channel_id: 810508141578027028
            }
        }
    }

    pub async fn run() {
        let app = Router::new()
            .route("/post-message", post(pass_message));

        let server = Server::build();
        println!("Server is running at {}", server.address);

        let listener = tokio::net::TcpListener::bind(server.address).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }

    async fn pass_message(Json(payload): Json<CreateMsg>) -> (StatusCode, Json<String>) {
        let msg: Msg = Msg {
            message: payload.content,
            attachment: None
        };

        let server = Server::build();

        if let Err(why) = push_message(String::from(msg.message), server.channel_id).await {
            return (StatusCode::BAD_REQUEST, Json(why.to_string()));
        };
        let message = String::from("Success");
        (StatusCode::OK, Json(message))
    }

    async fn push_message(content: String, ch_id: u64) -> Result<(), Box<dyn Error>> {
        let conf = Config::build();
        let http = Http::new(&conf?.token);
        let message = CreateMessage::new()
            .content(content);

        ChannelId::new(ch_id).send_message(http, message).await?;
        Ok(())
    }

    #[derive(Deserialize)]
    pub struct CreateMsg {
        pub content: String
    }
}
