pub trait TokenGenerator {
    fn generate(&self, byte_len: usize) -> String;
}
