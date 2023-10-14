use std::time::{SystemTime, UNIX_EPOCH};

use gossip_glomers_rs::{*, uuid::UUIDGenerator};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum GenerateUniqueIdPayload {
    Generate{},
    GenerateOk { id: String },
}

struct GenerateUniqueIdNode {
}

impl Node<(), GenerateUniqueIdPayload> for GenerateUniqueIdNode {
    fn from_init(
        state: (),
        init: Init,
        inject: std::sync::mpsc::Sender<Event<GenerateUniqueIdPayload, ()>>,
    ) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
        })
    }

    fn step(
        &mut self,
        input: Event<GenerateUniqueIdPayload, ()>,
        output: &mut std::io::StdoutLock,
    ) -> anyhow::Result<()> {
        match input {
            Event::Message(message) => {
                match message.body.payload {
                    GenerateUniqueIdPayload::Generate {} => {
                        let message_id = message.body.id;
                        let message = Message {
                            src: message.src,
                            dest: message.dest,
                            body: Body {
                                id: message.body.id,
                                in_reply_to: message.body.in_reply_to,
                                payload: GenerateUniqueIdPayload::GenerateOk {
                                    id: UUIDGenerator::new("./state.db".to_string()).generate().id,
                                },
                            },
                        };

                        message.into_reply(Some(&mut message_id.unwrap())).send(output)?;
                    }
                    GenerateUniqueIdPayload::GenerateOk { id } => {}
                }
            },

            Event::Injected(injected) => {},
            Event::EOF => {},
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
  main_loop::<(), GenerateUniqueIdNode, GenerateUniqueIdPayload, ()>(())?;
  Ok(())
}