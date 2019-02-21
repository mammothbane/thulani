## thulani

A Discord bot that:

- plays media in voice chat
- stores and plays back memes
- rolls dice

### Build
Install [Rust](https://rustup.rs/). Thulani builds on nightly.

You'll also need some libraries, but I don't have an exhaustive list. Off the top of my head, I know you'll need 
sodium-dev and openssl-dev, but there are probably a couple more. Just read the compile log and see what breaks.

The above should be enough to *build* thulani, but you'll also need `ffmpeg`, `youtube-dl`, and a postgres database to
run him.

Thulani *can* run on Windows, but I have thus far only managed this under mingw. I'm sure it's theoretically possible to
build him for MSVC, but I had significant issues with system libraries (openssl especially) when I tried this.

NB: The docker configuration is in a partially-complete state at the moment--it will not make your life easier.

### Run
```bash
cargo run --release
```

is the easiest way to do this.

### Postgres
Install postgres according to your distribution's relevant instructions, then create a database for thulani.

Install `diesel_cli` according to [these instructions](https://github.com/diesel-rs/diesel/tree/master/diesel_cli#installation).

You will (minimally) need to install `libpq` (possibly with headers as well) in order to do this.

Connect to your database and run

```postgresql
CREATE EXTENSION pgcrypto;
```

Then, in your shell (from thulani's root dir)

```bash
diesel migration run
```

### Configuration
Most of thulani's configuration is in his `.env` file. You will need to point him to the specific server he will
service, as well as his owner and the voice channel he will join when invoked.

You will also need to set up an app and a bot through Discord's developer portal. These will, respectively, provide you
with values for `THULANI_CLIENT_ID` and `THULANI_TOKEN`. 

`OP_ID` used to be for operators (people who could control thulani's playback), but I believe this no longer serves any
purpose. You should be able to drop this field.

If you want to use a different prefix or set of prefixes (so that he can be invoked with something other than `!thulani`),
the spot to do this is in `src/main.rs:run`.

### Disclaimer
I maintain this bot for my own personal Discord server and have no intention of developing him for more widespread use.
He is open source because I felt there was no reason to keep him private, not out of a desire to accept community input.
I don't want to discourage anyone from making PRs, but it's not likely I will pay them very much attention. A better use
of your time would almost always be to just fork him.
