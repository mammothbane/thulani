use std::sync::Arc;

use chrono::Duration;
use either::Either;
use serenity::{
    client::bridge::voice::ClientVoiceManager,
    model::{
        id::{
            ChannelId,
        },
    },
    prelude::*,
    voice::LockedAudio,
};
use typemap::Key;

pub use self::play_queue::PlayQueue;
pub use self::timeutil::parse_times;
pub use self::ytdl::*;

mod timeutil;
mod ytdl;
mod play_queue;

pub struct VoiceManager;

impl Key for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

impl VoiceManager {
    pub fn register(c: &mut Client) {
        let mut data = c.data.write();
        data.insert::<VoiceManager>(Arc::clone(&c.voice_manager));
    }
}

#[derive(Clone, Debug)]
pub struct PlayArgs {
    pub data: Either<String, Vec<u8>>,
    pub initiator: String,
    pub sender_channel: ChannelId,
    pub start: Option<Duration>,
    pub end: Option<Duration>,
}

#[derive(Clone)]
pub struct CurrentItem {
    pub init_args: PlayArgs,
    pub audio: LockedAudio,
}
