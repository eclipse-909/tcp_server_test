use tokio::sync::RwLockWriteGuard;
use {
    tokio::{
        net::{TcpListener, TcpStream},
        io::{AsyncReadExt, AsyncWriteExt},
        sync::RwLock,
        time::{timeout, Duration}
    },
    std::sync::Arc,
    crate::rpsp::RPSP,
};

pub trait Server : Send {
    fn get_listener(&self) -> &TcpListener;
    async fn get_running(&self) -> bool;
    
    async fn accept_clients(server: Arc<RwLock<Self>>) {
        println!("Server is now accepting clients");
        while server.read().await.get_running().await {
            match timeout(Duration::from_millis(1000), server.read().await.get_listener().accept()).await {
                Ok(Ok((socket, _))) => {
                    println!("Client connected to server");
                    let ref_server: Arc<RwLock<Self>> = Arc::clone(&server);
                    tokio::spawn(async move {ref_server.write().await.handle_client(Client::new(socket)).await});//TODO fix bug: not working for some reason
                }
                Ok(Err(err)) => {
                    eprintln!("Error accepting client: {:?}", err);
                }
                Err(_) => {
                    if !server.read().await.get_running().await {break;}
                }
            }
        }
        println!("Server is no longer accepting clients");
    }
    
    async fn handle_client(&mut self, client: Client) {//TODO fix bug: self is mutably borrowed multiple times
        let ref_client: Option<&mut Client> = self.add_client(client).await;
        match ref_client {
            Some(c) => {
                let mut buf: [u8; 1024] = [0; 1024];
                while c.is_connected {
                    match c.socket.read(&mut buf).await {
                        Ok(0) => {
                            self.disconnect_client(c).await;
                            println!("Client disconnected");
                            return;
                        }
                        Ok(n) => {self.parse_message(c, RPSP::from_bytes(&buf[0..n])).await;}
                        Err(e) => {
                            self.disconnect_client(c).await;
                            eprintln!("Client error - disconnecting: {e}", e = e);
                            return;
                        }
                    }
                }
            }
            None => {return;}
        }
    }
    
    async fn parse_message(&mut self, client: &mut Client, protocol: RPSP);
    
    fn send_message(client: &mut Client, protocol: &RPSP) {
        tokio::spawn(async {
            match client.socket.write_all(protocol.to_bytes()).await {
                Ok(()) => {
                    println!("Message sent to client successfully");
                }
                Err(e) => {
                    eprintln!("Message failed to send. Error: {e}", e = e);
                }
            }
        });
    }
    
    async fn add_client(&mut self, client: Client) -> Option<&mut Client>;
    
    async fn get_client(&mut self) -> Option<&mut Client>;
    
    async fn disconnect_client(&mut self, client: &mut Client);
    
    async fn shutdown_server(&mut self);
    
    fn admin_access_response(client: &mut Client, password_guess: &str) {
        if password_guess == ADMIN_PASSWORD {
            client.is_admin = true;
            Self::send_message(client, &RPSP::admin(&[1]));
        } else {
            Self::send_message(client, &RPSP::admin(&[0]));
        }
    }
}

const ADMIN_PASSWORD: &'static str = "h%408mm7Bb4QAj%y4*4t@F0*";

pub struct Client {
    pub socket: TcpStream,
    pub is_connected: bool,
    pub client_id: u8,
    pub team_id: u8,
    pub is_admin: bool
}

impl Client {
    pub fn new(socket: TcpStream) -> Self {
        return Client {
            socket,
            is_connected: true,
            client_id: 0,
            team_id: 0,
            is_admin: false
        }
    }
}