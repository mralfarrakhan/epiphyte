use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use clap::Parser;

#[derive(Serialize, Deserialize)]
struct Config {
    target_name: String,
    payload_path: PathBuf,
    port: Option<u16>,
    paths: Option<Vec<Path>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Path {
    pub name: String,
}

impl Config {
    fn read_config(config_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config = fs::read_to_string(config_path)?;
        let config: Self = toml::from_str(&config)?;

        Ok(config)
    }
}

#[derive(Parser)]
#[command(about, long_about=None)]
struct Cli {
    /// overrides config
    #[arg(short = 't', long)]
    target_name: Option<String>,

    /// overrides config
    #[arg(short, long)]
    payload_path: Option<PathBuf>,

    /// overrides config (default: 8070)
    #[arg(long)]
    port: Option<u16>,

    /// default: ./config.toml
    #[arg(short, long)]
    config_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub target_name: String,
    pub payload_path: PathBuf,
    pub port: u16,
    pub paths: Vec<Path>,
}

impl Options {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let cli = Cli::parse();

        let config_path = cli.config_path.unwrap_or_else(|| {
            println!("configuration file path is not set. using 'config.toml'");
            "config.toml".into()
        });
        let config = Config::read_config(config_path)?;

        let target_name = match cli.target_name {
            Some(v) => v,
            None => config.target_name,
        };

        let payload_path = match cli.payload_path {
            Some(v) => v,
            None => config.payload_path,
        };

        let port = match cli.port {
            Some(v) => v,
            None => match config.port {
                Some(v) => v,
                None => 8070,
            },
        };

        let paths = config.paths.ok_or("No paths is configured")?;

        let res = Self {
            target_name,
            payload_path,
            port,
            paths,
        };

        Ok(res)
    }
}
