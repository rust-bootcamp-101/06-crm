use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs::File};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub pk: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub db_url: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // 思考: 这里同时打开了三个文件去判断，会影响到效率(优化做法，按优先级打开，然后再判断是否需要打开下一个)，但这里是在程序初始化的时候去做，所以问题不大，可以接受
        // read from ./user_stat.yml, or /etc/config/user_stat.yml, or from env USER_STAT_CONFIG
        let ret: AppConfig = match (
            File::open("user_stat.yml"),
            File::open("/etc/config/user_stat.yml"),
            env::var("USER_STAT_CONFIG"),
        ) {
            (Ok(reader), _, _) => serde_yaml::from_reader(reader)?,
            (_, Ok(reader), _) => serde_yaml::from_reader(reader)?,
            (_, _, Ok(path)) => serde_yaml::from_reader(File::open(path)?)?,
            _ => bail!("Config file user_stat.yml not found"),
        };

        Ok(ret)
    }
}
