use {
	num_traits::{FromPrimitive},
	num_derive::{FromPrimitive, ToPrimitive}
};

pub struct RPSP {
	pub flag: Flag,
	destination: Destination,
	pub message: Option<Box<[u8]>>
}

impl RPSP {
	fn new(flag: Flag, destination: Destination, message: Option<&[u8]>) -> Self {
		return RPSP {
			flag,
			destination,
			message: message.map(|msg| Box::from(msg))
		};
	}
	
	pub fn from_bytes(bytes: &[u8]) -> Self {
		let first: u8 = bytes[0];
		let destination: u8 = first >> 2;
		return Self {
			flag: Flag::from_u8(first).unwrap(),
			destination: Destination::from_u8(destination).unwrap(),
			message: if bytes.len() > 1 {Some(Box::from(&bytes[1..]))} else {None}
		}
	}
	
	pub fn to_bytes(self) -> &'static [u8] {
		let first: u8 = ((self.flag as u8) << 2) | ((self.destination as u8) & 0b11);
		return match self.message {
			Some(msg) => {
				let mut bytes: Vec<u8> = msg.to_vec();
				bytes.insert(0, first);
				bytes.as_slice()
			}
			None => &[first]
		}
	}
	
	pub fn sync(client_id: u8) -> Self {return Self::new(Flag::SYNC, Destination::Server, Some(&[client_id]));}
	pub fn fin() -> Self {return Self::new(Flag::SYNC, Destination::Server, None);}
	pub fn err(message: &str) -> Self {return Self::new(Flag::SYNC, Destination::Server, Some(message.as_bytes()));}
	pub fn admin(message: &[u8]) -> Self {return Self::new(Flag::SYNC, Destination::Server, Some(message));}
	pub fn stop(response: bool) -> Self {return Self::new(Flag::SYNC, Destination::Server, Some(&[response as u8]));}
}

/**There can be a max of 2^6 or 64 flags*/
#[derive(FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Flag {
	//Common Flags
	SYNC,//Establish connection and clientId.
	FIN,//Close connection.
	ERR,//When there's an error, an error message is sent
	ADM,//Client requests admin permissions which allow access to certain functions
	STOP,//shuts down the server that the client is connected to. This can be dangerous and it's an admin-only flag
	MSET,//Game Flag - Broadcast a change in the settings of the game when the settings change. Server Flag - update the metadata of a game session.
	//Server Flags
	CREATE,//Creates a new game. Sent by client, server returns a GAME.
	PUB,//Gets a list of all public games and their metadata. It's kinda like a list of GAME.
	PRIV,//Request to join a private game by game code. Sent by client, server returns a GAME.
	GAME,//Sends the port number of the game session. Sent by server when client requests to join game.
	//Game Flags - Flags listed in the game code that aren't listed here are assumed to be Game Flags to be broadcast with no special action needed from the server.
	LEADEMP,//Request to change the leader empire
	DIFF,//Request to change the difficulty
	CTRL,//Request to change the human/ai controller
	TEAM,//Request to change team number
	USR,//Request a client's username
	KICK,//Kicks a client from the server
	CHAT,//Send message in chat
	READY,//Ready-up in lobby; ready to end turn in-game
	GDO,//Request a BaseGameData object
	EVENT//Broadcast a change in the state of the game when an event happens
}

impl Flag {
	pub fn from_u8(value: u8) -> Option<Self> {return FromPrimitive::from_u8(value);}
}

#[derive(FromPrimitive, ToPrimitive)]
#[repr(u8)]
enum Destination {Server, Broadcast, Team, Client}

impl Destination {
	pub fn from_u8(value: u8) -> Option<Self> {return FromPrimitive::from_u8(value);}
}