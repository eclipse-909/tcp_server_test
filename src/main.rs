use crate::{
    main_server::MainServer,
    server::Server
};

mod server;
mod main_server;
mod game_server;
mod rpsp;

#[tokio::main]
async fn main() {tokio::spawn(MainServer::accept_clients(MainServer::get().await)).await.unwrap();}