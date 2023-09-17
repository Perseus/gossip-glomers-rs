use gossip_glomers_rs::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Echo { echo: String },
    EchoOk { echo: String },
}

struct EchoNode {
    id: usize,
}

impl Node<(), Payload> for EchoNode {
    fn from_init(
        state: (),
        init: Init,
        inject: std::sync::mpsc::Sender<Event<Payload, ()>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self { id: 1 })
    }

    fn step(
            &mut self,
            input: Event<Payload, ()>,
            output: &mut std::io::StdoutLock,
        ) -> anyhow::Result<()> {
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    Ok(())
}
