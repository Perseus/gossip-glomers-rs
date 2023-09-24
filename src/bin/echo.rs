use gossip_glomers_rs::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum EchoPayload {
    Echo { echo: String },
    EchoOk { echo: String },
}

struct EchoNode {
    echo_message: String,
}

impl Node<(), EchoPayload> for EchoNode {
    fn from_init(
        state: (),
        init: Init,
        inject: std::sync::mpsc::Sender<Event<EchoPayload, ()>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            echo_message: "".to_string(),
        })
    }

    fn step(
        &mut self,
        input: Event<EchoPayload, ()>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::Message(message) => {
                match message.body.payload {
                    EchoPayload::Echo { echo } => {
                        self.echo_message = echo;
                        let message_id = message.body.id;
                        let message = Message {
                            src: message.src,
                            dest: message.dest,
                            body: Body {
                                id: message.body.id,
                                in_reply_to: message.body.in_reply_to,
                                payload: EchoPayload::EchoOk {
                                    echo: self.echo_message.clone(),
                                },
                            },
                        };

                        message.into_reply(Some(&mut message_id.unwrap())).send(output)?;
                    }
                    EchoPayload::EchoOk { echo } => {}
                }
            },

            Event::Injected(injected) => {},
            Event::EOF => {}
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    main_loop::<(), EchoNode, EchoPayload, ()>(())
}
