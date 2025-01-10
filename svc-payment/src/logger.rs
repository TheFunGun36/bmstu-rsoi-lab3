use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Logger, Root},
    Config, Handle,
};

pub fn init() -> Handle {
    let stdout = ConsoleAppender::builder().build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("app", LevelFilter::Debug))
        .build(Root::builder().appender("stdout").build(LevelFilter::Debug))
        .unwrap();

    log4rs::init_config(config).unwrap()
}
