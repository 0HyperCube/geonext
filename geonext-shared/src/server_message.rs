use serde::{Deserialize, Serialize};

use crate::territories::TerritoriesRLE;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
	AuthAccepted { username: String },
	Map(TerritoriesRLE),
	Error { message: String },
}
