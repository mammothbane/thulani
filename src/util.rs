use std::env;
use std::str::FromStr;

use serenity::model::permissions::Permissions;
use url::Url;

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
            REQUIRED_PERMS.bits(), env::var("THULANI_CLIENT_ID").expect("client ID was missing. please specify THULANI_CLIENT_ID in env or .env."),
        )
    ).unwrap();
}

pub fn must_env_lookup<T: FromStr>(s: &str) -> T {
    env::var(s).expect(&format!("missing env var {}", s))
        .parse::<T>().unwrap_or_else(|_| panic!(format!("bad format for {}", s)))
}
