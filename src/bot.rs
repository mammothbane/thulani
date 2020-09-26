use std::{
    sync::Mutex,
    fs::File,
    result::Result as StdResult,
};

use serenity::{
    prelude::*,
    model::{
        gateway::Ready,
        id::{
            ChannelId,
            MessageId,
        },
        channel::Message,
    },
    framework::StandardFramework,
};

use fnv::{
    FnvHashMap,
    FnvHashSet,
};

use chrono::Datelike;
use lazy_static::lazy_static;
use log::{
    debug,
    info,
    error,
    trace,
    warn,
};

use crate::{
    Result,
    Error,
    audio,
    util::OAUTH_URL,
    util::CtxExt,
    commands::register_commands,
    config::CONFIG,
};

struct Handler;
impl EventHandler for Handler {
    fn ready(&self, ctx: Context, r: Ready) {
        let guild = r.guilds.iter()
            .find(|g| g.id() == CONFIG.discord.guild());

        if guild.is_none() {
            info!("bot isn't in configured guild. join here: {:?}", OAUTH_URL.as_str());
        }

        #[cfg(debug_assertions)]
        let botname = "thulani (dev)";

        #[cfg(not(debug_assertions))]
        let botname = "thulani";

        guild.iter().for_each(|g| {
            if let Err(e) = g.id().edit_nickname(&ctx, Some(botname)) {
                error!("changing nickname: {:?}", e);
            }
        });
    }

    fn message_delete(&self, _ctx: Context, _channel_id: ChannelId, deleted_message_id: MessageId) {
        MESSAGE_WATCH.lock()
            .unwrap()
            .remove(&deleted_message_id);
    }
}

lazy_static! {
    static ref MESSAGE_WATCH: Mutex<FnvHashMap<MessageId, MessageId>> = Mutex::new(FnvHashMap::default());
    static ref PREFIXES: Vec<&'static str> = vec!["!thulani ", "!thulan ", "!thulando madando ", "!thulando "];
    static ref RESTRICTED_PREFIXES: Vec<&'static str> = vec!["!todd ", "!toddbert ", "!toddlani "];
    static ref ALL_PREFIXES: Vec<&'static str> = {
        let mut all_prefixes: Vec<&'static str> = vec![];
        all_prefixes.extend(PREFIXES.iter());
        all_prefixes.extend(RESTRICTED_PREFIXES.iter());
        all_prefixes
    };

    static ref RESTRICT_IDS: FnvHashSet<u64> = {
        let restrict_ids = File::open("restrict.json")
            .map_err(Error::from)
            .and_then(|f| serde_json::from_reader::<_, Vec<u64>>(f).map_err(Error::from));

        if let Err(ref e) = restrict_ids {
            warn!("opening restrict file: {}", e);
        }

        restrict_ids
            .unwrap_or_default()
            .into_iter()
            .collect::<FnvHashSet<_>>()
    };
}

fn framework() -> StandardFramework {
    let framework = StandardFramework::new()
        .configure(|c| c
            .allow_dm(false)
            .with_whitespace(true)
            .prefixes(ALL_PREFIXES.iter())
            .ignore_bots(true)
            .on_mention(None)
            .owners(vec![CONFIG.discord.owner()].into_iter().collect())
            .case_insensitivity(true)
        )
        .before(before_handle)
        .after(after_handle)
        .bucket("Standard", |b| {
            b.delay(1).limit(20).time_span(60)
        });

    register_commands(framework)
}

fn before_handle(ctx: &mut Context, message: &Message, cmd: &str) -> bool {
    debug!("got command '{}' from user '{}' ({})", cmd, message.author.name, message.author.id);

    if !message.guild_id.map_or(false, |x| x == CONFIG.discord.guild()) {
        info!("rejecting command '{}' from user '{}': wrong guild", cmd, message.author.name);
        return false;
    }

    if message.author.id == CONFIG.discord.owner() {
        return true;
    }

    let restricted_prefix = RESTRICTED_PREFIXES.iter()
        .any(|prefix| message.content.starts_with(prefix));

    if !restricted_prefix {
        return true;
    }

    const PERMITTED_WEEKDAY: chrono::Weekday = chrono::Weekday::Tue;

    let user_is_restricted = RESTRICT_IDS.contains(&message.author.id.0);
    let restrictions_flipped = chrono::Local::now().weekday() == PERMITTED_WEEKDAY;

    if user_is_restricted == restrictions_flipped {
        return true;
    }

    let reason = if !restrictions_flipped {
        "restricted prefix".to_owned()
    } else {
        format!("it is {:?}", PERMITTED_WEEKDAY)
    };

    info!("rejecting command '{}' from user '{}': {}", cmd, message.author.name, reason);

    match ctx.send_result(message.channel_id, "no", message.tts) {
        Err(e) => error!("sending restricted prefix response: {}", e),
        Ok(msg_id) => {
            let mut mp = MESSAGE_WATCH.lock().unwrap();
            mp.insert(message.id, msg_id);
        }
    }

    false
}

fn after_handle(ctx: &mut Context, msg: &Message, cmd: &str, err: StdResult<(), Error>) {
    match err {
        Ok(()) => {
            trace!("command '{}' completed successfully", cmd);
        },

        Err(e) => {
            if let Err(e) = msg.react(&ctx, "❌") {
                error!("reacting to failed message: {}", e);
            }

            if let Err(e) = ctx.send(msg.channel_id, "BANIC", msg.tts) {
                error!("sending BANIC: {}", e);
            }

            error!("error encountered handling command '{}': {:?}", cmd, e);
        }
    }
}

pub fn run() -> Result<()> {
    let token = &CONFIG.discord.auth.token;
    let mut client = Client::new(token, Handler)?;

    audio::VoiceManager::register(&mut client);
    audio::PlayQueue::register(&mut client);

    client.with_framework(framework());

    let shard_manager = client.shard_manager.clone();
    ctrlc::set_handler(move || {
        info!("shutting down");
        shard_manager.lock().shutdown_all();
    }).expect("unable to create SIGINT/SIGTERM handlers");

    client.start()
}
