use std::sync::Arc;

use either::Either;
use typemap::Key;
use chrono::Duration;

use serenity::{
    model::{
        id::ChannelId,
    },
    prelude::*,
    client::bridge::voice::ClientVoiceManager,
    voice::LockedAudio,
};

use crate::{
    must_env_lookup,
    Result,
};

pub use self::timeutil::parse_times;
pub use self::ytdl::ytdl;
pub use self::play_queue::PlayQueue;

mod timeutil;
mod ytdl;
mod play_queue;

pub trait CtxExt {
    fn currently_playing(&self) -> bool;
    fn users_listening(&self) -> Result<bool>;
}

impl CtxExt for Context {
    fn currently_playing(&self) -> bool {
        let queue_lock = self.data.lock().get::<PlayQueue>().cloned().unwrap();
        let play_queue = queue_lock.read().unwrap();
        play_queue.playing.is_none()
    }

    fn users_listening(&self) -> Result<bool> {
        let channel_id = ChannelId(must_env_lookup::<u64>("VOICE_CHANNEL"));
        let channel = channel_id.to_channel()?;
        let res = channel.guild()
            .and_then(|ch| ch.read().guild())
            .map(|g| (&g.read().voice_states)
                .into_iter()
                .any(|(_, state)| state.channel_id == Some(channel_id)))
            .unwrap_or(false);

        Ok(res)
    }
}

pub struct VoiceManager;

impl Key for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

impl VoiceManager {
    pub fn register(c: &mut Client) {
        let mut data = c.data.lock();
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
