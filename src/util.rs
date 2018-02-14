use serenity::model::permissions::Permissions;
use url::Url;

const REQUIRED_PERMS: Permissions = Permissions::EMBED_LINKS |
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

pub const OAUTH_URL: Url = Url::parse(
    concat!("https://discordapp.com/api/oauth2/authorize?scope=bot",
    "&permissions=", stringify!(REQUIRED_PERMS.bits()),
    "&client_id=", dotenv!("THULANI_CLIENT_ID"))).unwrap();
