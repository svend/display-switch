//
// Copyright © 2020 Haim Gelfenbeyn
// This code is licensed under MIT license (see LICENSE.txt for details)
//

use std::fs::File;

use anyhow::Result;
use simplelog::*;

use crate::configuration::Configuration;

pub fn init_logging(log_file: bool) -> Result<()> {
    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![
        TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed)];
    if log_file {
        loggers.push(WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create(Configuration::log_file_name()?)?,
        ))
    };
    CombinedLogger::init(loggers)?;

    Ok(())
}
