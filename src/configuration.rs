//
// Copyright © 2020 Haim Gelfenbeyn
// This code is licensed under MIT license (see LICENSE.txt for details)
//

use crate::input_source::InputSource;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Deserializer};
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum SwitchDirection {
    Connect,
    Disconnect,
}

#[derive(Debug, Deserialize, Copy, Clone)]
pub struct InputSources {
    // Note: Serde alias won't work here, because of https://github.com/serde-rs/serde/issues/1504
    // So cannot alias "on_usb_connect" to "monitor_input"
    pub on_usb_connect: Option<InputSource>,
    pub on_usb_disconnect: Option<InputSource>,
}

#[derive(Debug, Deserialize)]
struct PerMonitorConfiguration {
    monitor_id: String,
    #[serde(flatten)]
    input_sources: InputSources,
}

#[derive(Debug, Deserialize)]
pub struct Configuration {
    #[serde(deserialize_with = "Configuration::deserialize_usb_device")]
    pub usb_device: String,
    #[serde(flatten)]
    input_sources: InputSources,
    monitor1: Option<PerMonitorConfiguration>,
    monitor2: Option<PerMonitorConfiguration>,
    monitor3: Option<PerMonitorConfiguration>,
    monitor4: Option<PerMonitorConfiguration>,
    monitor5: Option<PerMonitorConfiguration>,
    monitor6: Option<PerMonitorConfiguration>,
}

impl fmt::Display for SwitchDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => write!(f, "connect"),
            Self::Disconnect => write!(f, "disconnect"),
        }
    }
}

impl PerMonitorConfiguration {
    fn matches(&self, monitor_id: &str) -> bool {
        monitor_id.to_lowercase().contains(&self.monitor_id.to_lowercase())
    }
}

impl InputSources {
    fn merge(&self, default: &Self) -> Self {
        Self {
            on_usb_connect: self.on_usb_connect.or(default.on_usb_connect),
            on_usb_disconnect: self.on_usb_disconnect.or(default.on_usb_disconnect),
        }
    }

    pub fn source(&self, direction: SwitchDirection) -> Option<InputSource> {
        match direction {
            SwitchDirection::Connect => self.on_usb_connect,
            SwitchDirection::Disconnect => self.on_usb_disconnect,
        }
    }
}

impl Configuration {
    pub fn load(config_file: Option<std::path::PathBuf>) -> Result<Self> {
        let config_file_name = if let Some(name) = config_file {
            name
        } else {
            Self::config_file_name()?
        };
        let mut settings = config::Config::default();
        settings
            .merge(config::File::from(config_file_name.clone()))?
            .merge(config::Environment::with_prefix("DISPLAY_SWITCH"))?;
        let config = settings.try_into::<Self>()?;
        info!("Configuration loaded ({:?}): {:?}", config_file_name, config);
        Ok(config)
    }

    fn deserialize_usb_device<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(s.to_lowercase())
    }

    pub fn config_file_name() -> Result<std::path::PathBuf> {
        let config_dir = if cfg!(target_os = "macos") {
            dirs::preference_dir().ok_or_else(|| anyhow!("Config directory not found"))?
        } else {
            dirs::config_dir()
                .ok_or_else(|| anyhow!("Config directory not found"))?
                .join("display-switch")
        };
        std::fs::create_dir_all(&config_dir)
            .with_context(|| format!("failed to create directory: {:?}", config_dir))?;
        Ok(config_dir.join("display-switch.ini"))
    }

    pub fn log_file_name() -> Result<std::path::PathBuf> {
        let log_dir = if cfg!(target_os = "macos") {
            dirs::home_dir()
                .ok_or_else(|| anyhow!("Home directory not found"))?
                .join("Library")
                .join("Logs")
                .join("display-switch")
        } else {
            dirs::data_local_dir()
                .ok_or_else(|| anyhow!("Data-local directory not found"))?
                .join("display-switch")
        };
        std::fs::create_dir_all(&log_dir).with_context(|| format!("failed to create directory: {:?}", log_dir))?;
        Ok(log_dir.join("display-switch.log"))
    }

    pub fn configuration_for_monitor(&self, monitor_id: &str) -> InputSources {
        // Find a matching per-monitor config, if there is any
        let per_monitor_config = [
            &self.monitor1,
            &self.monitor2,
            &self.monitor3,
            &self.monitor4,
            &self.monitor5,
            &self.monitor6,
        ]
        .iter()
        .find_map(|config| {
            config
                .as_ref()
                .and_then(|config| if config.matches(monitor_id) { Some(config) } else { None })
        });
        // Merge global config as needed
        per_monitor_config.map_or(self.input_sources, |config| {
            config.input_sources.merge(&self.input_sources)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::ConfigError;
    use config::FileFormat::Ini;

    #[test]
    fn test_log_file_name() {
        let file_name = Configuration::log_file_name();
        assert!(file_name.is_ok());
        assert!(file_name.unwrap().ends_with("display-switch.log"))
    }

    fn load_test_config(config_str: &str) -> Result<Configuration, ConfigError> {
        let mut settings = config::Config::default();
        settings.merge(config::File::from_str(config_str, Ini)).unwrap();
        settings.try_into::<Configuration>()
    }

    #[test]
    fn test_usb_device_deserialization() {
        let config = load_test_config(
            r#"
            usb_device = "dead:BEEF"
            on_usb_connect = "DisplayPort2"
        "#,
        )
        .unwrap();
        assert_eq!(config.usb_device, "dead:beef")
    }

    #[test]
    fn test_symbolic_input_deserialization() {
        let config = load_test_config(
            r#"
            usb_device = "dead:BEEF"
            on_usb_connect = "DisplayPort2"
            on_usb_disconnect = DisplayPort1
        "#,
        )
        .unwrap();
        assert_eq!(config.input_sources.on_usb_connect.unwrap().value(), 0x10);
        assert_eq!(config.input_sources.on_usb_disconnect.unwrap().value(), 0x0f);
    }

    #[test]
    fn test_decimal_input_deserialization() {
        let config = load_test_config(
            r#"
            usb_device = "dead:BEEF"
            on_usb_connect = 22
            on_usb_disconnect = 33
        "#,
        )
        .unwrap();
        assert_eq!(config.input_sources.on_usb_connect.unwrap().value(), 22);
        assert_eq!(config.input_sources.on_usb_disconnect.unwrap().value(), 33);
    }

    #[test]
    fn test_hexadecimal_input_deserialization() {
        let config = load_test_config(
            r#"
            usb_device = "dead:BEEF"
            on_usb_connect = "0x10"
            on_usb_disconnect = "0x20"
        "#,
        )
        .unwrap();
        assert_eq!(config.input_sources.on_usb_connect.unwrap().value(), 0x10);
        assert_eq!(config.input_sources.on_usb_disconnect.unwrap().value(), 0x20);
    }

    #[test]
    fn test_per_monitor_config() {
        let config = load_test_config(
            r#"
            usb_device = "dead:BEEF"
            on_usb_connect = "0x10"
            on_usb_disconnect = "0x20"

            [monitor1]
            monitor_id = 123
            on_usb_connect = 0x11

            [monitor2]
            monitor_id = 45
            on_usb_connect = 0x12
            on_usb_disconnect = 0x13
        "#,
        )
        .unwrap();

        // When no specific monitor matches, use the global defaults
        assert_eq!(
            config.configuration_for_monitor("333").on_usb_connect.unwrap().value(),
            0x10
        );
        // Matches monitor #1, and it should use its "on-connect" and global "on-disconnect"
        assert_eq!(
            config.configuration_for_monitor("1234").on_usb_connect.unwrap().value(),
            0x11
        );
        assert_eq!(
            config
                .configuration_for_monitor("1234")
                .on_usb_disconnect
                .unwrap()
                .value(),
            0x20
        );
        // Matches monitor #2, and it should use its "on-connect" and "on-disconnect" values
        assert_eq!(
            config.configuration_for_monitor("2345").on_usb_connect.unwrap().value(),
            0x12
        );
        assert_eq!(
            config
                .configuration_for_monitor("2345")
                .on_usb_disconnect
                .unwrap()
                .value(),
            0x13
        );
    }
}
