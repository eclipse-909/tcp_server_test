use {
	std::{
		collections::HashMap,
		sync::Arc
	},
	tokio::{
		net::TcpListener,
		io::AsyncWriteExt,
		sync::{RwLock, RwLockWriteGuard}
	},
	crate::{
		game_server::GameServer,
		server::{Server, Client},
		rpsp::{RPSP, Flag}
	}
};

pub struct MainServer {
	listener: TcpListener,
	running: RwLock<bool>,
	game_servers: RwLock<HashMap<u16, Arc<RwLock<GameServer>>>>,
	clients: RwLock<Vec<Client>>
}

impl Server for MainServer {
	fn get_listener(&self) -> &TcpListener {return &self.listener;}
	async fn get_running(&self) -> bool {return *self.running.read().await;}
	
	async fn parse_message(&mut self, client: &mut Client, protocol: RPSP) {
		match protocol.flag {
			Flag::SYNC => {
				println!("Client has connected");
				client.client_id = 1;
				Self::send_message(client, &RPSP::sync(1));
			}
			Flag::FIN => {
				println!("Client has disconnected");
			}
			Flag::ERR => {
				println!("Client had an error: {}", std::str::from_utf8(&protocol.message.unwrap_or(Box::from([0xff]))).unwrap_or("Invalid error message"));
			}
			Flag::ADM => {
				println!("Client requesting admin powers");
				Self::admin_access_response(client, std::str::from_utf8(&protocol.message.unwrap_or(Box::from([0xff]))).unwrap_or("Invalid message"));
			}
			Flag::STOP => {
				if client.is_admin {
					self.shutdown_server().await;
				}
			}
			Flag::MSET => {}
			Flag::CREATE => {}
			Flag::PUB => {}
			Flag::PRIV => {}
			Flag::GAME => {}
			_ => {
				println!("Flag not valid for main server {}", protocol.flag as u8);
				Self::send_message(client, &RPSP::err("Received an invalid flag for the main server"));
			}
		}
	}
	
	async fn add_client(&mut self, mut client: Client) -> Option<&mut Client> {
		client.client_id = 1;
		let mut lock: &RwLockWriteGuard<Vec<Client>> = &self.clients.write().await;
		lock.push(client);
		return lock.last_mut();
	}
	
	async fn get_client(&mut self) -> Option<&mut Client> {
		return &*self.clients.write().await.last_mut();//TODO fix bug: calling write() creates a new lock guard local variable who's lifetime ends on return.
	}
	
	async fn disconnect_client(&mut self, client: &mut Client) {
		client.is_connected = false;
		client.socket.shutdown().await.expect("Server should have shut down");
		self.clients.write().await.retain(|c| c as *const _ != client as *const _);
	}
	
	async fn shutdown_server(&mut self) {
		*self.running.write().await = false;
		while let Some(mut client) = {let x = self.clients.write().await.pop(); x} {
			Server::disconnect_client(self, &mut client).await;
		}
		let keys: Vec<u16> = self.game_servers.read().await.keys().cloned().collect();
		for key in keys {
			Self::remove_game_server(&key).await;
		}
	}
}

static mut MAIN_SERVER: Option<Arc<RwLock<MainServer>>> = None;

impl MainServer {
	pub async fn get() -> Arc<RwLock<Self>> {
		unsafe {
			if MAIN_SERVER.is_none() {
				MAIN_SERVER = Some(Arc::new(RwLock::new(Self {
					listener: TcpListener::bind("0.0.0.0:42069").await.unwrap(),
					running: RwLock::new(true),
					game_servers: RwLock::new(HashMap::new()),
					clients: RwLock::new(Vec::new())
				})));
			}
			return Arc::clone(MAIN_SERVER.as_ref().unwrap());
		}
	}
	
	async fn create_game_server() {
		println!("Game server created");
		let game_server: GameServer = GameServer::new().await;
		let port: u16 = game_server.get_listener().local_addr().unwrap().port().clone();
		let game_server: Arc<RwLock<GameServer>> = Arc::new(RwLock::new(game_server));
		let ref_game_server: Arc<RwLock<GameServer>> = Arc::clone(&game_server);
		Self::get().await.read().await.game_servers.write().await.insert(port.clone(), game_server);
		tokio::spawn(async move {GameServer::accept_clients(ref_game_server).await});
	}
	
	pub async fn remove_game_server(port: &u16) {
		match Self::get().await.read().await.game_servers.write().await.remove(port) {
			Some(game_server) => {
				println!("Removed game server on port {}", port);
				*game_server.read().await.running.write().await = false;
				for i in 1..=8 {
					match game_server.read().await.clients.read().await.get(&i) {
						Some(_) => {game_server.read().await.clients.write().await.remove(&i);}
						None => {}
					}
				}
			}
			None => {println!("Game server not found for removal");}
		}
	}
}