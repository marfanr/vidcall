use std::sync::Arc;

pub struct RoomConnection{

}

#[derive(Clone, Debug)]
pub struct Room{
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub connection: Vec<Arc<str>>
}

// impl Room {

// }