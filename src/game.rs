use std::{
    iter,
    result::Result as StdResult,
    str::{
        self,
        FromStr,
    },
};

use failure::{
    err_msg,
    Error,
};
use fnv::{
    FnvHashMap,
    FnvHashSet,
};
use itertools::Itertools;
use serenity::{
    framework::standard::{
        Args,
        StandardFramework,
    },
    model::{
        channel::Message,
        guild::Guild,
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
    static ref STEAM_API_KEY: String = must_env_lookup("STEAM_API_KEY");
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
        .command("updategame", |c| c
            .known_as("updategaem")
            .desc("update your games on the spreadsheet")
            .exec(updategaem)
        )
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
struct UserInfo {
    name: String,

    #[serde(flatten)]
    profile: ProfileInfo,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ProfileInfo {
    #[serde(rename = "steam")]
    steam_id: Option<u64>,

    #[serde(rename = "discord")]
    discord_user_id: u64,
}

lazy_static! {
    static ref USER_MAP_STR: &'static str = str::from_utf8(&include_bytes!("../user_id_mapping.json")[..]).unwrap();

    static ref USER_INFO_MAP: FnvHashMap<String, ProfileInfo> = {
        let v: Vec<UserInfo> = serde_json::from_str(*USER_MAP_STR).unwrap();

        v.into_iter()
            .map(|ui| {
                let UserInfo { name, profile } = ui;

                (name, profile)
            })
            .collect::<FnvHashMap<_, _>>()
    };

    static ref DISCORD_MAP: FnvHashMap<UserId, String> = {
        USER_INFO_MAP.clone().into_iter()
            .map(|(name, profile)| {
                (UserId(profile.discord_user_id), name)
            })
            .collect::<FnvHashMap<_, _>>()
    };

    static ref STEAM_MAP: FnvHashMap<UserId, u64> = {
        USER_INFO_MAP.clone().into_iter()
            .filter_map(|(_, profile)| {
                profile.steam_id.map(|sid| (UserId(profile.discord_user_id), sid))
            })
            .collect::<FnvHashMap<_, _>>()
    };

    static ref ALPHABET: Vec<char> = (0..26).map(|x| (x + b'a') as char).collect();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd)]
enum GameStatus {
    Installed,
    NotInstalled,
    NotOwned,
    Unknown,
}

impl FromStr for GameStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        use std::char;

        if s.starts_with("y") {
            Ok(GameStatus::Installed)
        } else if s.starts_with("n/i") {
            Ok(GameStatus::NotInstalled)
        } else if s.starts_with("n") {
            Ok(GameStatus::NotOwned)
        } else if s.chars().all(char::is_whitespace) {
            Ok(GameStatus::Unknown)
        } else {
            Err(err_msg(format!("unexpected status '{}'", s)))
        }
    }
}

fn installedgame(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    game(ctx, msg, args, GameStatus::Installed)
}

fn ownedgame(ctx: &mut Context, msg: &Message, args: Args) -> Result<()> {
    game(ctx, msg, args, GameStatus::NotInstalled)
}

#[derive(Copy, Clone, Debug, Fail, PartialEq, Eq, Hash)]
enum UserLookupError {
    #[fail(display = "too many possible options ({}) for query", _0)]
    Ambiguous(usize),

    #[fail(display = "user wasn't found in the guild")]
    NotFound,
}

fn get_user_id<S: AsRef<str>>(g: &Guild, s: S) -> StdResult<UserId, UserLookupError> {
    let s = s.as_ref().trim_start_matches("@").to_lowercase();

    if let Some(info) = USER_INFO_MAP.get(&s) {
        return Ok(UserId(info.discord_user_id));
    }

    let nicks = g.members_nick_containing(&s, false, false);

    {
        let exact_match = nicks
            .iter()
            .find(|m| m.user.read().name.to_lowercase() == s);

        if let Some(m) = exact_match {
            return Ok(m.user_id());
        }
    }

    let usernames = g.members_username_containing(&s, false, false);

    {
        let exact_match = usernames
            .iter()
            .find(|m| m.user.read().name.to_lowercase() == s);

        if let Some(m) = exact_match {
            return Ok(m.user_id());
        }
    }

    let opts = nicks.into_iter().chain(usernames.into_iter())
        .map(|member| member.user_id())
        .collect::<FnvHashSet<_>>();

    match opts.len() {
        0 => Err(UserLookupError::NotFound),
        1 => Ok(opts.into_iter().next().unwrap()),
        x => Err(UserLookupError::Ambiguous(x))
    }
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
        .filter_map(|u| {
            use std::borrow::Borrow;

            let possible = get_user_id(guild.borrow(), &u);

            debug!("parsed userid {:?}", possible);

            match possible {
                Err(UserLookupError::NotFound) => {
                    let _ = send(msg.channel_id, &format!("didn't recognize {}", &u), msg.tts);
                    None
                },
                Ok(x) => Some(x),
                Err(UserLookupError::Ambiguous(x)) => {
                    let _ = send(msg.channel_id, &format!("too many matches ({}) for {}", x, &u), msg.tts);
                    None
                },
            }
        })
        .filter_map(|uid| {
            let res = DISCORD_MAP.get(&uid).map(|s| s.to_lowercase());

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
                    DISCORD_MAP.get(uid).map(|s| s.to_lowercase())
                } else { None }
            })
            .collect::<FnvHashSet<_>>();
    }

    if inferred && users.len() < 2 || !inferred && users.len() < 1 {
        info!("too few known users to make game comparison");
        send(msg.channel_id, "yer too lonely", msg.tts)?;
        return Ok(());
    }

    let data = load_spreadsheet()?;

    let user_indexes = (0..data.len())
        .filter_map(|i| {
            let user = data[i][0].to_lowercase();

            if users.contains(&user) {
                Some((user, i))
            } else { None }
        })
        .collect::<FnvHashMap<_, _>>();

    let data_ref = &data;
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
                    let status = &data_ref[*col][i].parse::<GameStatus>().unwrap_or(GameStatus::Unknown);
                    let game = &data_ref[0][i];

                    game_map.get_mut(status).unwrap().insert(game);
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

    let mut games_formatted = games_in_common.iter().sorted_by(|a, b| {
        a.to_lowercase().cmp(&b.to_lowercase())
    }).join("\n");

    if games_formatted.is_empty() {
        games_formatted = "**LITERALLY NOTHING**".to_owned();
    }

    send(msg.channel_id, &games_formatted, msg.tts)
}

fn load_spreadsheet() -> Result<Vec<Vec<String>>> {
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

    let resp = resp.json::<Resp>()?;

    Ok(resp.value_ranges.into_iter().next().unwrap().values)
}

fn to_spreadsheet_alpha(mut x: usize) -> String {
    let mut result = String::new();

    result.push(ALPHABET[x % 26]);
    x /= 26;

    while x != 0 {
        result.push(ALPHABET[x % 26]);
        x /= 26;
    }

    result.chars().rev().collect()
}

fn updategaem(_ctx: &mut Context, msg: &Message, mut args: Args) -> Result<()> {
    use regex::Regex;

    let arg_user = args.single_quoted::<String>();

    let user = if arg_user.is_err() {
        msg.author.id.clone()
    } else {
        use std::borrow::Borrow;

        let guild = msg.channel_id.to_channel()?
            .guild()
            .ok_or(err_msg("couldn't find guild"))?;

        let guild = guild.read()
            .guild()
            .ok_or(err_msg("couldn't find guild"))?;

        let guild = guild
            .read();

        get_user_id(guild.borrow(), arg_user.unwrap()).map_err(Error::from)?
    };

    debug!("parsed userid {:?}", user);

    let username = match DISCORD_MAP.get(&user) {
        Some(s) => s,
        None => return send(msg.channel_id, "WHO THE FUCK ARE YE", msg.tts),
    };

    let steam_id = match STEAM_MAP.get(&user) {
        Some(u) => u,
        None => return send(msg.channel_id, "WHO ARE YE ON STEAM", msg.tts),
    };

    let spreadsheet = load_spreadsheet()?;

    let user_column = (0..spreadsheet.len())
        .find(|x| spreadsheet[*x][0].to_lowercase() == username.to_lowercase());

    let user_column = match user_column {
        Some(c) => &spreadsheet[c][1..],
        None => return send(msg.channel_id, "YER NOT IN THE SPREADSHEET", msg.tts),
    };

    lazy_static! {
        static ref APPID_REGEX: Regex = Regex::new(r#"(?i)^\s*app\s*id\s*$"#).unwrap();
    }

    let appid_column = (0..spreadsheet.len())
        .find(|x| APPID_REGEX.is_match(&spreadsheet[*x][0]));

    let appid_column = match appid_column {
        Some(c) => &spreadsheet[c][1..],
        None => {
            error!("didn't find an appid column in the spreadsheet");
            return send(msg.channel_id, "SPREADSHEET BROKE", msg.tts)
        },
    };

    let missing_appids = (0..user_column.len())
        .filter_map(|x| user_column[x].parse::<GameStatus>().ok().map(|s| (x, s)))
        .filter(|(_, s)| *s == GameStatus::Unknown || *s == GameStatus::NotOwned)
        .filter_map(|(x, _)| appid_column
            .get(x)
            .and_then(|s| s.parse::<u64>().ok().map(|appid| (appid, x))));

    let mut u = Url::parse("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1")?;

    u.query_pairs_mut()
        .append_pair("key", &*STEAM_API_KEY)
        .append_pair("include_played_free_games", "1")
        .append_pair("steamid", &steam_id.to_string());

    #[derive(Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
    struct SteamResp {
        response: SteamInner,
    }

    #[derive(Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
    struct SteamInner {
        games: Vec<SteamGameEntry>,
    }

    #[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct SteamGameEntry {
        #[serde(rename = "appid")]
        app_id: u64,

        #[serde(rename = "playtime_forever")]
        play_time: u64,
    }

    let games_owned= reqwest::get(u)?
        .json::<SteamResp>()?
        .response.games
        .into_iter()
        .map(|ge| ge.app_id)
        .collect::<FnvHashSet<_>>();

    let found_games = missing_appids
        .filter_map(|(ai, x)| if games_owned.contains(&ai) {
            Some(&spreadsheet[0][x + 1])
        } else {
            None
        })
        .join("\n");

    if found_games.len() > 0 {
        send(msg.channel_id, &format!("{} games owned on steam that are missing from the list:\n{}",
                                      found_games.chars().filter(|x| *x == '\n').count() + 1,
                                      found_games), msg.tts)
    } else {
        send(msg.channel_id, "up to date", msg.tts)
    }
}