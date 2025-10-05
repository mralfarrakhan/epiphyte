use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use clap::Parser;

#[derive(Serialize, Deserialize, Default)]
struct Config {
    target_name: Option<String>,
    payload_path: Option<PathBuf>,
    port: Option<u16>,
    timeout: Option<u64>,
    paths: Option<Vec<Map>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
struct Map {
    pub name: String,
    pub symbol: Option<String>,
}

impl Config {
    fn read_config(config_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
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

    /// show additional info
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub target_name: String,
    pub payload_path: PathBuf,
    pub port: u16,
    pub timeout: u64,
    pub paths: Vec<Identifier>,
    pub is_verbose: bool,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub symbol: String,
}

impl Options {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let cli = Cli::parse();

        let config_path = cli.config_path.unwrap_or_else(|| {
            println!("[WARNING] configuration file path is not set.");
            println!("[WARNING] looking for ./config.toml.");
            "config.toml".into()
        });
        let config = match Config::read_config(&config_path) {
            Ok(v) => v,
            Err(err) => {
                eprintln!(
                    "[ERROR] cannot read config in '{}': {:?}.",
                    config_path.display(),
                    err
                );
                Default::default()
            }
        };

        let target_name = cli.target_name.or(config.target_name).ok_or(
            "target name is defined in neither configuration file nor command line arguments.",
        )?;

        let payload_path = cli.payload_path.or(config.payload_path).ok_or(
            "payload path is defined in neither configuration file nor command line arguments",
        )?;

        let port = cli.port.or(config.port).unwrap_or(8070);

        let timeout = config.timeout.unwrap_or(500);

        let paths = match config.paths {
            Some(v) => v,
            None => {
                println!("[WARNING] no paths defined in configuration file.");
                Default::default()
            }
        };
        let paths = paths
            .iter()
            .map(|x| {
                let name = x.name.as_str();
                Identifier {
                    name: name.into(),
                    symbol: x.symbol.clone().unwrap_or(name.into()),
                }
            })
            .collect();

        let is_verbose = cli.verbose;

        let res = Self {
            target_name,
            payload_path,
            port,
            timeout,
            paths,
            is_verbose,
        };

        Ok(res)
    }
}
