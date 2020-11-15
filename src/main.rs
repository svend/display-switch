//
// Copyright © 2020 Haim Gelfenbeyn
// This code is licensed under MIT license (see LICENSE.txt for details)
//
#![windows_subsystem = "windows"]

#[macro_use]
extern crate log;

use anyhow::Result;

mod app;
mod configuration;
mod display_control;
mod input_source;
mod logging;
mod platform;
mod usb;

fn main() -> Result<()> {
    let config_file = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let app = app::App::new(Some(config_file), false)?;
    app.run()?;

    Ok(())
}
