use crate::color::Color;
use getopts::Options;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::env;
use std::fs::read_to_string;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum OutputMode {
    All,
    Active,
}

impl Default for OutputMode {
    fn default() -> Self {
        OutputMode::All
    }
}

fn default_scale() -> u32 {
    1
}
fn default_background() -> Color {
    Color::new(0.0, 0.0, 0.0, 0.9)
}
fn default_cmd() -> String {
    "".to_string()
}
fn default_headline() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}
fn default_prompt() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}
fn default_prompt_err() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}
fn default_border() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub output_mode: OutputMode,
    #[serde(default = "default_scale")]
    pub scale: u32,
    #[serde(default = "default_background")]
    pub background: Color,
    #[serde(default = "default_headline")]
    pub headline: Color,
    #[serde(default = "default_prompt")]
    pub prompt: Color,
    #[serde(default = "default_prompt_err")]
    pub prompt_err: Color,
    #[serde(default = "default_border")]
    pub border: Color,
    #[serde(default = "default_cmd")]
    pub command: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            output_mode: Default::default(),
            scale: 1,
            background: Color::new(0.0, 0.0, 0.0, 0.9),
            headline: Color::new(1.0, 1.0, 1.0, 1.0),
            prompt: Color::new(1.0, 1.0, 1.0, 1.0),
            prompt_err: Color::new(1.0, 1.0, 1.0, 1.0),
            border: Color::new(1.0, 1.0, 1.0, 1.0),
            command: "".to_string(),
        }
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

pub fn read_config() -> Config {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("c", "config", "config file to use", "CONFIG_FILE");
    opts.optopt("e", "command", "command to run", "COMMAND");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        std::process::exit(0);
    }

    let mut config: Config = match read_to_string(
        matches
            .opt_str("config")
            .unwrap_or_else(|| "/etc/greetd/wlgreet.toml".to_string()),
    ) {
        Ok(s) => match toml::from_str(&s) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Unable to parse configuration file: {:?}", e);
                eprintln!("Please fix the configuration file and try again.");
                std::process::exit(1);
            }
        },
        Err(_) => Default::default(),
    };

    config.command = matches.opt_get_default("command", config.command).unwrap();

    config
}
