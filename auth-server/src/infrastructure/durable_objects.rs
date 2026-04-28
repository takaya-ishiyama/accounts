use worker::*;

#[durable_object]
pub struct AuthDurableObject {
    state: State,
    env: Env,
}

impl DurableObject for AuthDurableObject {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, _req: Request) -> Result<Response> {
        Response::ok("AuthDurableObject")
    }
}
