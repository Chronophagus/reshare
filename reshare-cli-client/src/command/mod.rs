pub mod config;
pub mod get;
pub mod list;
pub mod put;

use super::cli::{ConfigArgs, GetArgs, ListArgs, PutArgs};
use super::Result;
use anyhow::Context;
use reqwest::blocking as http;
use reqwest::Url;

const CONFIG_FILE_NAME: &str = "reshare-url";

type Configuration = Url;

fn configure(server_addr: &str) -> Result<()> {
    let config_file_path = get_config_file_path();

    Url::parse(server_addr)?;
    std::fs::write(config_file_path, server_addr.trim()).context("Error writing configuration file")
}

fn load_configuration() -> Result<Configuration> {
    let config_file_path = get_config_file_path();
    let conf = std::fs::read_to_string(config_file_path).context(
        "Reading configuration file. Did you run `reshare config` to configure server url?",
    )?;

    if conf.is_empty() {
        anyhow::bail!("Configuration file is empty");
    }

    Ok(Url::parse(&conf)?)
}

fn get_config_file_path() -> std::path::PathBuf {
    dirs_next::config_dir()
        .map(|path| path.join(CONFIG_FILE_NAME))
        .unwrap_or_else(|| std::path::Path::new("/").join(CONFIG_FILE_NAME))
}
