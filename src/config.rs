use serenity::{
    model::id::{
        GuildId,
        UserId,
        ChannelId,
    },
};

use dotenv::dotenv;
use lazy_static::lazy_static;
use envconfig::Envconfig;

lazy_static! {
    pub static ref CONFIG: Config = {
        dotenv().ok();

        Config::init().unwrap()
    };
}

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "DATABASE_URL")]
    pub db_string: String,

    #[envconfig(from = "MAX_HIST")]
    pub max_hist: usize,

    #[envconfig(from = "DEFAULT_HIST")]
    pub default_hist: usize,

    #[envconfig(from = "STEAM_API_KEY")]
    pub steam_api_key: String,

    pub discord: DiscordConfig,

    pub sheets: SheetsConfig,
}

#[derive(Envconfig)]
pub struct DiscordConfig {
    pub auth: DiscordAuth,

    #[envconfig(from = "TARGET_GUILD")]
    guild: u64,

    #[envconfig(from = "OWNER_ID")]
    owner: u64,

    #[envconfig(from = "VOICE_CHANNEL")]
    voice_channel: u64,
}

impl DiscordConfig {
    #[inline]
    pub fn guild(&self) -> GuildId {
        self.guild.into()
    }

    #[inline]
    pub fn owner(&self) -> UserId {
        self.owner.into()
    }

    #[inline]
    pub fn voice_channel(&self) -> ChannelId {
        self.voice_channel.into()
    }
}

#[derive(Envconfig)]
pub struct DiscordAuth {
    #[envconfig(from = "THULANI_CLIENT_ID")]
    pub client_id: u64,

    #[envconfig(from = "THULANI_TOKEN")]
    pub token: String,
}

#[derive(Envconfig)]
pub struct SheetsConfig {
    #[envconfig(from = "SHEETS_API_KEY")]
    pub api_key: String,

    #[envconfig(from = "SPREADSHEET_ID")]
    pub spreadsheet: String,

    #[envconfig(from = "MAX_SHEET_COLUMN")]
    pub max_column: String,
}