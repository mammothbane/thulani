use super::*;

command!(meme(_ctx, msg) {
    send(msg.channel_id, "I am not yet capable of memeing", msg.tts)?;
});
