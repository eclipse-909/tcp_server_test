use {
	tokio::{
		net::TcpListener,
		io::AsyncWriteExt,
		sync::{RwLock, RwLockWriteGuard}
	},
	std::collections::HashMap,
	crate::{
		server::{Server, Client},
		main_server::MainServer,
		rpsp::{RPSP, Flag}
	}
};

pub struct GameServer {
	listener: TcpListener,
	pub running: RwLock<bool>,
	pub clients: RwLock<HashMap<u8, Client>>,
	client_id_pool: RwLock<[bool; 8]>
}

impl Server for GameServer {
	fn get_listener(&self) -> &TcpListener {return &self.listener;}
	async fn get_running(&self) -> bool {return *self.running.read().await;}
	
	async fn parse_message(&mut self, client: &mut Client, protocol: RPSP) {
		match protocol.flag {
			Flag::SYNC => {
				println!("Client has connected");
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
			Flag::LEADEMP => {}
			Flag::DIFF => {}
			Flag::CTRL => {}
			Flag::TEAM => {}
			Flag::USR => {}
			Flag::KICK => {}
			Flag::CHAT => {}
			Flag::READY => {}
			Flag::GDO => {}
			Flag::EVENT => {}
			_ => {
				println!("Flag not valid for game server");
				Self::send_message(client, &RPSP::err("Received an invalid flag for the game server"));
			}
		}
	}
	
	async fn add_client(&mut self, mut client: Client) -> Option<&mut Client> {
		return match self.take_id().await {
			Some(id) => {
				client.client_id = id;
				Self::send_message(&mut client, &RPSP::sync(id));
				let mut lock: RwLockWriteGuard<HashMap<u8, Client>> = self.clients.write().await;
				lock.insert(id, client);
				lock.get_mut(&id)//TODO fix bug: returns reference to data owned by function
			}
			None => {
				Server::disconnect_client(self, &mut client).await;
				None
			}
		}
	}
	
	async fn disconnect_client(&mut self, client: &mut Client) {
		client.is_connected = false;
		client.socket.shutdown().await.expect("Server should have shut down");
		self.clients.write().await.remove(&client.client_id);
		self.return_id(client.client_id).await;
		if self.clients.read().await.is_empty() {
			self.shutdown_server().await;
		}
	}
	
	async fn shutdown_server(&mut self) {
		MainServer::remove_game_server(&self.listener.local_addr().unwrap().port()).await;
	}
}

impl GameServer {
	pub async fn new() -> Self {
		println!("Game server listening on random port");
		return GameServer {
			listener: TcpListener::bind("0.0.0.0:0").await.unwrap(),
			running: RwLock::new(true),
			clients: RwLock::new(HashMap::new()),
			client_id_pool: RwLock::new([true; 8])
		};
	}
	
	async fn take_id(&mut self) -> Option<u8> {
		for (id, available) in self.client_id_pool.write().await.iter_mut().enumerate() {
			if *available {
				*available = false;
				return Some((id + 1) as u8);
			}
		}
		return None;
	}
	
	async fn return_id(&mut self, id: u8) {
		let index = (id - 1) as usize;
		if 0 <= index && index < 8 {
			self.client_id_pool.write().await[index] = true;
		}
	}
}