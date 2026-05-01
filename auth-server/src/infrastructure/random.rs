use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

use crate::domain::auth::token::TokenGenerator;

pub fn random_base64url(byte_len: usize) -> String {
    let mut buf = vec![0u8; byte_len];
    getrandom::getrandom(&mut buf).expect("getrandom failed");
    URL_SAFE_NO_PAD.encode(&buf)
}

pub struct GetRandomTokenGenerator;

impl TokenGenerator for GetRandomTokenGenerator {
    fn generate(&self, byte_len: usize) -> String {
        random_base64url(byte_len)
    }
}
