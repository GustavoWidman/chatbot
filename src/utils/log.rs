use colog::format::CologStyle;
use colored::Colorize;
use env_logger::Builder;
use log::{Level, LevelFilter};

struct CustomLevelTokens;

impl CologStyle for CustomLevelTokens {
    fn level_token(&self, level: &Level) -> &str {
        match *level {
            Level::Error => "ERR",
            Level::Warn => "WRN",
            Level::Info => "INF",
            Level::Debug => "DBG",
            Level::Trace => "TRC",
        }
    }

    fn prefix_token(&self, level: &Level) -> String {
        format!(
            "{}{}{} {}{}{}",
            "[".blue().bold(),
            chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S.%6f")
                .to_string()
                .white()
                .bold(),
            "]".blue().bold(),
            "[".blue().bold(),
            self.level_color(level, self.level_token(level)),
            "]".blue().bold()
        )
    }
}

pub struct Logger;

impl Logger {
    pub fn init(level: Option<LevelFilter>) {
        Builder::new()
            .filter(Some("chatbot"), level.unwrap_or(LevelFilter::Trace))
            .filter(Some("rig-core"), LevelFilter::Trace)
            .filter(Some("reqwest"), LevelFilter::Warn)
            .filter(Some("serenity"), LevelFilter::Warn)
            .filter(Some("poise"), LevelFilter::Warn)
            .target(env_logger::Target::Stdout)
            .format(colog::formatter(CustomLevelTokens))
            .write_style(env_logger::WriteStyle::Always)
            .init();
    }
}
