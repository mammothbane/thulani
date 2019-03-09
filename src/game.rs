use std::iter;

use failure::err_msg;
use fnv::FnvHashMap;
use itertools::Itertools;
use serenity::{
    framework::standard::{
        Args,
        StandardFramework,
    },
    model::{
        channel::Message,
        id::UserId,
    },
    prelude::*,
};
use url::Url;

use crate::{
    commands::send,
    must_env_lookup,
    Result,
    VOICE_CHANNEL_ID,
};

lazy_static! {
    static ref SHEETS_API_KEY: String = must_env_lookup("SHEETS_API_KEY");
    static ref SPREADSHEET_ID: String = must_env_lookup("SPREADSHEET_ID");
    static ref MAX_SHEET_COLUMN: String = must_env_lookup("MAX_SHEET_COLUMN");
}

pub fn register(s: StandardFramework) -> StandardFramework {
    s
        .command("game", |c| c
            .known_as("gaem")
            .known_as("installedgaem")
            .known_as("installedgame")
            .desc("what game should we play?")
            .exec(installedgame)
        )
        .command("ownedgame", |c| c
            .known_as("ownedgaem")
            .desc("what games does everyone have?")
            .exec(ownedgame)
        )
}

lazy_static! {
    static ref USER_MAP: FnvHashMap<UserId, String> = {
        use serde_json::Value;
        use std::str;

        let map_bytes = include_bytes!("../user_id_mapping.json");

        let v: Value = serde_json::from_str(str::from_utf8(&map_bytes[..]).unwrap()).unwrap();
        match v {
            Value::Object(m) => {
                m.iter()
                    .map(|(k, v)| match v {
                         Value::Number(n) => (UserId(n.as_u64().unwrap()), k.clone()),
                         _ => panic!("non-number in user id mapping"),
                    })
                    .collect()
            },
            _ => panic!("couldn't read user id mapping"),
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd)]
enum GameStatus {
    Installed,
    NotInstalled,
    NotOwned,
    Unknown,
}

fn installedgame(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    game(ctx, msg, args, GameStatus::Installed)
}

fn ownedgame(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    game(ctx, msg, args, GameStatus::NotInstalled)
}

fn game(_ctx: &mut Context, msg: &Message, args: Args, min_status: GameStatus) -> Result<()> {
    use fnv::{
        FnvHashMap,
        FnvHashSet,
    };

    let guild = msg.channel_id.to_channel()?
        .guild()
        .ok_or(err_msg("couldn't find guild"))?;

    let guild = guild.read()
        .guild()
        .ok_or(err_msg("couldn't find guild"))?;

    let guild = guild
        .read();

    let user_args = if args.rest().is_empty() {
        Vec::new()
    } else {
        args.multiple_quoted::<String>()?
    };

    let mut users = user_args
        .into_iter()
        .map(|u| u.trim_start_matches("@").to_owned())
        .filter_map(|u| {
            let mut possible = guild.members_nick_containing(&u, false, false);
            possible.extend(guild.members_username_containing(&u, false, false));

            let possible = possible.into_iter()
                .map(|member| member.user_id())
                .collect::<FnvHashSet<_>>();

            match possible.len() {
                0 => {
                    let _ = send(msg.channel_id, &format!("didn't recognize {}", u), msg.tts);
                    None
                },
                1 => Some(possible.into_iter().next().unwrap()),
                x => {
                    let _ = send(msg.channel_id, &format!("too many matches ({}) for {}", x, u), msg.tts);
                    None
                },
            }
        })
        .filter_map(|uid| {
            let res = USER_MAP.get(&uid).map(|s| s.to_lowercase());

            if let None = res {
                let _ = info!("user {} is not recognized", uid);
            }

            res
        })
        .collect::<FnvHashSet<_>>();

    let inferred = users.len() == 0;

    if users.len() == 0 {
        let pairs = guild
            .voice_states
            .iter()
            .filter_map(|(uid, voice)| {
                voice.channel_id.map(|cid| (*uid, cid))
            })
            .collect::<FnvHashMap<_, _>>();

        let channel = pairs.get(&msg.author.id).unwrap_or(&*VOICE_CHANNEL_ID);

        users = pairs
            .iter()
            .filter_map(|(uid, cid)| {
                if cid == channel {
                    USER_MAP.get(uid).map(|s| s.to_lowercase())
                } else { None }
            })
            .collect::<FnvHashSet<_>>();
    }

    if inferred && users.len() < 2 || !inferred && users.len() < 1 {
        info!("too few known users to make game comparison");
        send(msg.channel_id, "yer too lonely", msg.tts)?;
        return Ok(());
    }

    let mut u = Url::parse(
        &format!("https://sheets.googleapis.com/v4/spreadsheets/{}/values:batchGet", *SPREADSHEET_ID))?;

    u.query_pairs_mut()
        .append_pair("ranges", &format!("a1:{}", &*MAX_SHEET_COLUMN))
        .append_pair("valueRenderOption", "FORMATTED_VALUE")
        .append_pair("majorDimension", "COLUMNS")
        .append_pair("key", &*SHEETS_API_KEY);

    let req = reqwest::Request::new(reqwest::Method::GET, u);

    let client = reqwest::Client::new();

    let mut resp = client.execute(req)?;

    #[derive(Deserialize)]
    struct Resp {
        #[serde(rename = "valueRanges")]
        value_ranges: Vec<Inner>,
    }

    #[derive(Deserialize)]
    struct Inner {
        values: Vec<Vec<String>>,
    }

    let data = &resp.json::<Resp>()?.value_ranges[0].values;

    let user_indexes = (0..data.len())
        .filter_map(|i| {
            let user = data[i][0].to_lowercase();

            if users.contains(&user) {
                Some((user, i))
            } else { None }
        })
        .collect::<FnvHashMap<_, _>>();

    let user_games = user_indexes
        .iter()
        .map(|(user, col)| {
            let empty_hash_set: FnvHashSet<_> = vec![].into_iter().collect();

            let mut game_map = vec! [
                (GameStatus::Installed, empty_hash_set.clone()),
                (GameStatus::NotInstalled, empty_hash_set.clone()),
                (GameStatus::NotOwned, empty_hash_set.clone()),
                (GameStatus::Unknown, empty_hash_set),
            ]
                .into_iter()
                .collect::<FnvHashMap<_, _>>();

            (1..data[*col].len())
                .for_each(|i| {
                    let status = &data[*col][i];

                    let game = &data[0][i];
                    if status.starts_with("y") {
                        game_map.get_mut(&GameStatus::Installed).unwrap().insert(game);
                    } else if status.starts_with("n/i") {
                        game_map.get_mut(&GameStatus::NotInstalled).unwrap().insert(game);
                    } else if status.starts_with("n") {
                        game_map.get_mut(&GameStatus::NotOwned).unwrap().insert(game);
                    } else {
                        game_map.get_mut(&GameStatus::Unknown).unwrap().insert(game);
                    }
                });

            (user, game_map)
        })
        .collect::<FnvHashMap<_, _>>();

    let statuses = vec![GameStatus::Installed, GameStatus::NotOwned, GameStatus::NotInstalled, GameStatus::Unknown]
        .into_iter()
        .filter(|s| s <= &min_status)
        .collect::<Vec<_>>();

    let mut games_in_common = {
        let game_map = user_games.values().nth(0).unwrap();

        statuses
            .iter()
            .fold(iter::empty().collect::<FnvHashSet<_>>(), |acc, s| {
                acc.union(&game_map[s]).cloned().collect()
            })
    };

    for (_user, game_map) in user_games.iter() {
        let relevant_games = statuses
            .iter()
            .fold(iter::empty().collect::<FnvHashSet<_>>(), |acc, s| {
                acc.union(&game_map[s]).cloned().collect()
            });

        games_in_common = games_in_common.intersection(&relevant_games).cloned().collect();
    }

    let games_formatted = games_in_common.iter().sorted_by(|a, b| {
        a.to_lowercase().cmp(&b.to_lowercase())
    }).join("\n");

    send(msg.channel_id, &games_formatted, msg.tts)
}
