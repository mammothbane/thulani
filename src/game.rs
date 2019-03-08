use failure::err_msg;
use oauth2::Config;
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

use crate::{
    commands::send,
    must_env_lookup,
    Result,
    VOICE_CHANNEL_ID,
};

lazy_static! {
    static ref SHEETS_CLIENT_ID: String = must_env_lookup("SHEETS_CLIENT_ID");
    static ref SHEETS_SECRET: String = must_env_lookup("SHEETS_CLIENT_SECRET");
    static ref SPREADSHEET_ID: String = must_env_lookup("SPREADSHEET_ID");
}

#[cfg(debug_assertions)] const REDIRECT_URL: &'static str = "http://localhost:8080";
#[cfg(not(debug_assertions))] const REDIRECT_URL: &'static str = "https://somali-derp.com/thulani_redirect";

pub fn register(s: StandardFramework) -> StandardFramework {
    use std::{
        thread,
        time::Duration,
    };

    thread::spawn(|| {
        thread::sleep(Duration::from_secs(10));

        loop {
            debug!("starting token maintenance");
            if let Err(e) = maintain_token() {
                error!("maintaining google access token: {}", e);
            }
            debug!("token maintenance complete");

            thread::sleep(Duration::from_secs(60 * 2));
        }
    });

    s.command("game", |c| c
        .known_as("gaem")
        .desc("what game should we play?")
        .exec(game)
    )
}

fn game(_ctx: &mut Context, msg: &Message, _args: Args) -> Result<()> {
    use std::collections::HashSet;
    use fnv::FnvHashMap;

    let guild = msg.channel_id.to_channel()?
        .guild()
        .ok_or(err_msg("couldn't find guild"))?;

    let guild = guild.read()
        .guild()
        .ok_or(err_msg("couldn't find guild"))?;

    let guild = guild
        .read();

    let pairs = guild
        .voice_states
        .iter()
        .filter_map(|(uid, voice)| {
            voice.channel_id.map(|cid| (*uid, cid))
        })
        .collect::<FnvHashMap<_, _>>();

    let channel = pairs.get(&msg.author.id).unwrap_or(&*VOICE_CHANNEL_ID);
    let mut users = HashSet::new();

    pairs.iter().for_each(|(uid, cid)| {
        if cid == channel {
            users.insert(*uid);
        }
    });

//    if users.len() < 2 {
//        info!("too few users in voice chat to make game comparison");
//        send(msg.channel_id, "yer too lonely", msg.tts)?;
//        return Ok(());
//    }

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

    use url::Url;

    let mut u = Url::parse(
        &format!("https://sheets.googleapis.com/v4/spreadsheets/{}/values:batchGet", *SPREADSHEET_ID))?;

    u.query_pairs_mut()
        .append_pair("ranges", "a1:p")
        .append_pair("valueRenderOption", "UNFORMATTED_VALUE")
        .append_pair("majorDimension", "COLUMNS");

    let oauth_token = get_oauth_token()?;

    let mut req = reqwest::Request::new(reqwest::Method::GET, u);
    req.headers_mut().insert("Authorization", reqwest::header::HeaderValue::from_str(&format!("Bearer {}", oauth_token))?);

    let client = reqwest::Client::new();

    let mut resp = client.execute(req)?;

    #[derive(Deserialize)]
    struct Resp {
        #[serde(rename = "valueRanges")]
        value_ranges: Inner,
    }

    #[derive(Deserialize)]
    struct Inner {
        values: Vec<Vec<String>>,
    }

    let data = resp.json::<Resp>()?.value_ranges.values;

    use itertools::Itertools;
    info!("data: {}", data.iter().map(|row| row.iter().join(" ")).join("\n"));

    Ok(())
}

lazy_static! {
    static ref CONFIG: Config = Config::new(
        SHEETS_CLIENT_ID.as_ref(),
        SHEETS_SECRET.as_ref(),
        "https://accounts.google.com/o/oauth2/v2/auth",
        "https://www.googleapis.com/oauth2/v4/token",
    )
    .add_scope("https://www.googleapis.com/auth/spreadsheets.readonly")
    .set_redirect_url(REDIRECT_URL);
}

fn get_oauth_token() -> Result<String> {
    use std::{
        net::TcpListener,
    };

    use url::Url;
    use chrono;

    use diesel::{
        NotFound,
        result::Error as DieselError,
    };

    use crate::db;


    lazy_static! {
        static ref AUTH_URL: Url = {
            let mut u = CONFIG.authorize_url();
            u.query_pairs_mut()
                .append_pair("access_type", "offline");

            u
        };
    }

    #[cfg(debug_assertions)]
    const PORT: u16 = 8080;

    #[cfg(not(debug_assertions))]
    const PORT: u16 = 8981;

    let conn = db::connection()?;

    let token = db::GoogleOAuthToken::latest(&conn);

    match token {
        Ok(t) => return Ok(t.token),
        Err(e) => {
            if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                info!("no token found in database");
            } else {
                return Err(e);
            }
        }
    }

    eprintln!("please navigate to {} in your browser", AUTH_URL.as_str());

    let listener = TcpListener::bind(&format!("127.0.0.1:{}", PORT))?;

    const ATTEMPTS: usize = 10;
    let code = listener.incoming()
        .filter_map(|s| s.ok())
        .map(|mut stream| {
            use std::io::{
                BufReader,
                BufRead,
                Write,
            };

            let mut request_line = String::new();

            {
                let mut reader = BufReader::new(&stream);
                reader.read_line(&mut request_line).ok()?;
            }

            let url =
                Url::parse(&format!("http://localhost{}", request_line.split_whitespace().nth(1)?)).ok()?;

            let code = url.query_pairs()
                .find(|(key, _)| key == "code")
                .map(|(_, code)| code.into_owned());

            let message = "all set";
            let resp = format!(
                "HTTP/1.1 20 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message,
            );

            stream.write_all(resp.as_bytes()).ok()?;

            code
        })
        .take(ATTEMPTS)
        .find(|x| !x.is_none());

    let code = match code {
        None => return Err(err_msg(format!("couldn't acquire oauth code from google after {} attempts", ATTEMPTS))),
        Some(c) => c.unwrap(),
    };

    let token = CONFIG.exchange_code(code)?;

    if token.expires_in.is_none() || token.refresh_token.is_none() {
        return Err(err_msg("token expiration or refresh token was missing"));
    }

    let now = chrono::Utc::now().naive_utc();
    let new_expiration = token.expires_in
        .map(|exp_sec| now + chrono::Duration::seconds(exp_sec.into()))
        .unwrap();

    let result = db::GoogleOAuthToken::create(&conn, token.access_token, token.refresh_token.unwrap(), new_expiration)?;

    Ok(result.token)
}

fn maintain_token() -> Result<()> {
    use diesel::{
        Connection,
        result::Error as DieselError,
        NotFound,
    };

    use chrono;

    use crate::db;

    let conn = db::connection()?;

    conn.transaction(|| {
        let latest_token = db::GoogleOAuthToken::latest(&conn);
        let latest_token = match latest_token {
            Ok(t) => t,
            Err(e) => {
                if let Some(NotFound) = e.downcast_ref::<DieselError>() {
                    info!("maintaining google auth: no token to refresh found in database");
                    return Ok(());
                }

                return Err(e);
            }
        };

        let now = chrono::Utc::now().naive_utc();
        let diff = latest_token.expiration - now;

        if diff > chrono::Duration::minutes(10) {
            info!("token has {} minutes remaining: not refreshing", diff.num_minutes());
            return Ok(());
        }

        let new_token = CONFIG.exchange_refresh_token(latest_token.refresh_token)?;

        if new_token.refresh_token.is_none() || new_token.expires_in.is_none() {
            return Err(err_msg("refreshed token missing refresh token or expiration"));
        }

        info!("received new token from google");

        let new_expiration = new_token.expires_in
            .map(|exp_sec| now + chrono::Duration::seconds(exp_sec.into()))
            .unwrap();

        db::GoogleOAuthToken::create(&conn,
                                     new_token.access_token,
                                     new_token.refresh_token.unwrap(),
                                     new_expiration)?;

        Ok(())
    })
}
