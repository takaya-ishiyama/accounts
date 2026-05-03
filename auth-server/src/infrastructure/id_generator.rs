use crate::{domain::id_generator::IdGenerator, infrastructure::ids::new_uuid_v7};

pub struct UuidV7Generator;

impl IdGenerator for UuidV7Generator {
    fn new_id(&self) -> String {
        new_uuid_v7()
    }
}
