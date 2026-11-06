use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub username: String,
    pub game_dir: String,
    pub ram_min_mb: u32,
    pub ram_max_mb: u32,
    pub java_path: String,
    pub jvm_args: String,
    pub api_proxy_url: String,
    pub selected_version: String,
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        let default_game_dir = Self::default_minecraft_dir()
            .to_string_lossy()
            .to_string();

        let default_java = if cfg!(target_os = "windows") {
            "javaw".to_string()
        } else {
            "java".to_string()
        };

        Self {
            username: "Player".to_string(),
            game_dir: default_game_dir,
            ram_min_mb: 1024,
            ram_max_mb: 4096,
            java_path: default_java,
            jvm_args: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200 -XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC".to_string(),
            api_proxy_url: "http://100.64.0.13:8000".to_string(),
            selected_version: "1.20.4".to_string(),
            window_width: 1280,
            window_height: 720,
        }
    }
}

impl LauncherConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let local_cfg = parent.join("catrinth-config.json");
                return local_cfg;
            }
        }
        PathBuf::from("catrinth-config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<LauncherConfig>(&content) {
                    return cfg;
                }
            }
        }
        let def = Self::default();
        let _ = def.save();
        def
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let json_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Ошибка сериализации конфига: {}", e))?;
        fs::write(&path, json_str)
            .map_err(|e| format!("Не удалось сохранить файл настроек: {}", e))?;
        Ok(())
    }

    pub fn default_minecraft_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            if let Some(appdata) = dirs::data_dir() {
                return appdata.join(".minecraft");
            }
        } else if cfg!(target_os = "macos") {
            if let Some(home) = dirs::home_dir() {
                return home.join("Library/Application Support/minecraft");
            }
        } else {
            if let Some(home) = dirs::home_dir() {
                return home.join(".minecraft");
            }
        }
        PathBuf::from(".minecraft")
    }

    pub fn mods_dir(&self) -> PathBuf {
        Path::new(&self.game_dir).join("mods")
    }

    pub fn versions_dir(&self) -> PathBuf {
        Path::new(&self.game_dir).join("versions")
    }

    pub fn libraries_dir(&self) -> PathBuf {
        Path::new(&self.game_dir).join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        Path::new(&self.game_dir).join("assets")
    }
}
