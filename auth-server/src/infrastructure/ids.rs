#[cfg(target_arch = "wasm32")]
pub fn new_uuid_v7() -> String {
    let ms = js_sys::Date::now() as u64;
    let ts = uuid::Timestamp::from_unix(uuid::NoContext, ms / 1000, ((ms % 1000) * 1_000_000) as u32);
    uuid::Uuid::new_v7(ts).to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn new_uuid_v7() -> String {
    uuid::Uuid::now_v7().to_string()
}
