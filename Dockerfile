FROM rustlang/rust:nightly

RUN apt-get update -yqq && apt-get install -yqq libsodium-dev

WORKDIR /usr/src/thulani
COPY src ./src
COPY Cargo.toml ./

RUN cargo fetch

COPY .env ./

RUN cargo build --release

FROM python:3
RUN pip install youtube-dl
RUN apt-get update -yqq && apt-get install -yqq libsodium18 ffmpeg

COPY --from=0 /usr/src/thulani/target/release/thulani .
COPY .env ./

CMD ["./thulani"]
