use std::{
    env,
    str::FromStr,
};

use dotenv;
use serenity::{
    client::Context,
    model::{
        id::{
            ChannelId,
            MessageId,
        },
        permissions::Permissions,
    }
};
use url::Url;

use lazy_static::lazy_static;

use crate::{
    audio::PlayQueue,
    Result,
};

pub trait CtxExt {
    fn currently_playing(&self) -> bool;
    fn users_listening(&self) -> Result<bool>;
    fn send<A: AsRef<str>>(&self, channel: ChannelId, text: A, tts: bool) -> Result<()>;
    fn send_result<A: AsRef<str>>(&self, channel: ChannelId, text: A, tts: bool) -> Result<MessageId>;
}

impl CtxExt for Context {
    fn currently_playing(&self) -> bool {
        let queue_lock = self.data.read().get::<PlayQueue>().cloned().unwrap();
        let play_queue = queue_lock.read().unwrap();
        play_queue.playing.is_some()
    }

    fn users_listening(&self) -> Result<bool> {
        let channel_id = ChannelId(must_env_lookup::<u64>("VOICE_CHANNEL"));
        let channel = channel_id.to_channel(self)?;
        let res = channel.guild()
            .and_then(|ch| ch.read().guild(self))
            .map(|g| (&g.read().voice_states)
                .into_iter()
                .any(|(_, state)| state.channel_id == Some(channel_id)))
            .unwrap_or(false);

        Ok(res)
    }

    #[inline]
    fn send<A: AsRef<str>>(&self, channel: ChannelId, text: A, tts: bool) -> Result<()> {
        self.send_result(channel, text, tts).map(|_| ())
    }

    #[inline]
    fn send_result<A: AsRef<str>>(&self, channel: ChannelId, text: A, tts: bool) -> Result<MessageId> {
        let result = channel.send_message(self, |m| m.content(text.as_ref()).tts(tts))?;
        Ok(result.id)
    }
}

lazy_static! {
    static ref REQUIRED_PERMS: Permissions = Permissions::EMBED_LINKS |
        Permissions::READ_MESSAGES |
        Permissions::ADD_REACTIONS |
        Permissions::SEND_MESSAGES |
        Permissions::SEND_TTS_MESSAGES |
        Permissions::MENTION_EVERYONE |
        Permissions::USE_EXTERNAL_EMOJIS |
        Permissions::CONNECT |
        Permissions::SPEAK |
        Permissions::CHANGE_NICKNAME |
        Permissions::USE_VAD |
        Permissions::ATTACH_FILES;
}

lazy_static! {
    pub static ref OAUTH_URL: Url = Url::parse(
        &format!(
            "https://discordapp.com/api/oauth2/authorize?scope=bot&permissions={}&client_id={}",
            REQUIRED_PERMS.bits(), dotenv!("THULANI_CLIENT_ID"),
        )
    ).unwrap();
}

pub fn must_env_lookup<T: FromStr>(s: &str) -> T {
    env::var(s).expect(&format!("missing env var {}", s))
        .parse::<T>().unwrap_or_else(|_| panic!(format!("bad format for {}", s)))
}
