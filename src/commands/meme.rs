use rand::{thread_rng, distributions::{Weighted, WeightedChoice, Distribution}};
use serenity::http::AttachmentType;
use serenity::builder::CreateMessage;
use diesel::PgConnection;

use super::*;
use super::playback::CtxExt;

use ::db::*;

#[derive(Clone, Copy, Debug)]
enum MemeType {
    Text,
    Image,
    Audio,
}

static mut MEME_WEIGHTS: [Weighted<MemeType>; 3] = [
    Weighted { weight: 1, item: MemeType::Text },
    Weighted { weight: 1, item: MemeType::Image },
    Weighted { weight: 1, item: MemeType::Audio },
];

static mut TTS_WEIGHTS: [Weighted<bool>; 2] = [
    Weighted { weight: 4, item: false },
    Weighted { weight: 1, item: true }
];

command!(meme(ctx, msg, args) {
    let ch = msg.channel_id;

    if args.len() == 0 {
        rand_meme(ctx, msg)?;
        return Ok(());
    }
});

fn rand_meme(ctx: &Context, message: &Message) -> Result<()> {
    let conn = connection()?;

    let should_audio = ctx.currently_playing() && ctx.users_listening()?;
    let dist: WeightedChoice<'static, MemeType> = if should_audio {
        WeightedChoice::new(unsafe { &mut MEME_WEIGHTS })
    } else {
        WeightedChoice::new(unsafe { &mut MEME_WEIGHTS[..2] })
    };

    match dist.sample(&mut thread_rng()) {
        MemeType::Text => {
            let mut text_meme = rand_text(&conn)?;

            let mut ctr = 0;
            while !should_audio && text_meme.audio_id.is_some() {
                text_meme = rand_text(&conn)?;

                ctr += 1;
                if ctr > 10 {
                    warn!("looped 10 times trying to find a non-audio text meme");
                    return Ok(());
                }
            }

            send_meme(ctx, &text_meme, &conn, message)?;
        },
        MemeType::Image => send_meme(ctx, &rand_image(&conn)?, &conn, message)?,
        MemeType::Audio => send_meme(ctx, &rand_audio(&conn)?, &conn, message)?,
    }

    Ok(())
}


fn send_meme(ctx: &Context, t: &Meme, conn: &PgConnection, msg: &Message) -> Result<()> {
    let image = t.image(conn);
    let audio = t.audio(conn);

    let dist = WeightedChoice::new(unsafe { &mut TTS_WEIGHTS });

    let create_msg = |m: CreateMessage| {
        let ret = m
            .tts(dist.sample(&mut thread_rng()));

        match t.content {
            Some(ref text) => ret.content(text),
            None => ret
        }
    };

    match image {
        Some(image) => msg.channel_id.send_files(vec!(AttachmentType::Bytes((&image?.data, &t.title))), create_msg)?,
        None => msg.channel_id.send_message(create_msg)?,
    };

    // note: slight edge-case race condition here: there could have been something queued since we
    //  checked whether anything was playing. not a significant negative impact and unlikely, so i'm
    //  not worrying about it
    if let Some(audio) = audio {
        let audio = audio?;
        let queue_lock = ctx.data.lock().get::<PlayQueue>().cloned().unwrap();
        let mut play_queue = queue_lock.write().unwrap();

        play_queue.queue.push_front(PlayArgs{
            initiator: msg.author.name.clone(),
            data: ::either::Right(audio.data.clone()),
            sender_channel: msg.channel_id,
        });
    }

    Ok(())
}

pub fn db_fallback(ctx: &mut Context, msg: &Message, s: &str) -> Result<()> {


    Ok(())
}
