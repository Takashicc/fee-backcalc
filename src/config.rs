use anyhow::Context as _;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::AppConfig;

pub fn config_dir() -> PathBuf {
    config_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    let base_dir = env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    base_dir.join("fee-backcalc").join("config.json")
}

pub fn load_config() -> AppConfig {
    load_config_from_path(&config_path()).unwrap_or_default()
}

pub fn load_config_from_path(path: &Path) -> anyhow::Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("設定ファイルの読み込みに失敗しました: {}", path.display()))?;
    let config = serde_json::from_str(&content)
        .with_context(|| format!("設定ファイルの解析に失敗しました: {}", path.display()))?;
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> anyhow::Result<()> {
    save_config_to_path(config, &config_path())
}

pub fn open_config_directory() -> anyhow::Result<PathBuf> {
    let dir = config_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("設定ディレクトリの作成に失敗しました: {}", dir.display()))?;
    open_directory(&dir)?;
    Ok(dir)
}

pub fn save_config_to_path(config: &AppConfig, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("設定ディレクトリの作成に失敗しました: {}", parent.display())
        })?;
    }

    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)
        .with_context(|| format!("設定ファイルの保存に失敗しました: {}", path.display()))?;
    Ok(())
}

fn open_directory(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(path);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };

    command
        .spawn()
        .with_context(|| format!("設定フォルダを開けませんでした: {}", path.display()))?;
    Ok(())
}
