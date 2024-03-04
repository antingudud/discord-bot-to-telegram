use std::process;
use std::sync::Arc;

use serenity::prelude::*;

use discord::Config;
use discord::Handler;

use discord::server::server;

#[tokio::main]
async fn main() {
    let config = Config::build().unwrap_or_else(|err| {
        println!("Error building config: {}", err);
        process::exit(1);
    });
    let handler: Handler = Handler;

    let mut client = Client::builder(config.token.clone(), GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS).event_handler(handler).await.expect("Err creating client.");
    let global_data = Arc::new(RwLock::new(discord::DataWrapper {
        disc_id: None,
        tele_id: None,
        context: None
    }));
    let server = server::Server::build(config, global_data.clone());

    {
        let mut data = client.data.write().await;

        data.insert::<server::ServerWrapper>(Arc::new(RwLock::new(server::ServerWrapper {
            server: server.clone()
        })));

        data.insert::<discord::DataWrapper>(global_data.clone());
    }

    let _ = client.start().await;
}
