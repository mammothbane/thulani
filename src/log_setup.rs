use crate::{Result, Error};

use fern::colors::{Color, ColoredLevelConfig};

pub fn init() -> Result<()> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::BrightBlue)
        .trace(Color::BrightMagenta);

    fern::Dispatch::new()
        .level_for("serenity::voice::connection", log::LevelFilter::Error)
        .chain(fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "{} [{}] [{}] {}",
                    chrono::Local::now().format("%_m/%_d/%y %l:%M:%S%P"),
                    colors.color(record.level()),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Warn)
            .level_for("thulani", log::LevelFilter::Debug)
            .chain(std::io::stdout())
        )
        .chain(fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{} [{}] [{}] {}",
                    chrono::Local::now().format("%_m/%_d/%y %l:%M:%S%P"),
                    record.level(),
                    record.target(),
                    message
                ))
            })
            .level(log::LevelFilter::Info)
            .level_for("thulani", log::LevelFilter::Trace)
            .chain(fern::log_file("thulani.log").expect("problem creating log file"))
        )
        .apply()
        .map_err(Error::from)
}