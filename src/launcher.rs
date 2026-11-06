use crate::config::LauncherConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct InstalledMod {
    pub filename: String,
    pub size_formatted: String,
    pub path: PathBuf,
}

pub struct MinecraftLauncher;

impl MinecraftLauncher {
    pub fn get_installed_versions(config: &LauncherConfig) -> Vec<String> {
        let versions_dir = config.versions_dir();
        let mut versions = Vec::new();

        if versions_dir.exists() {
            if let Ok(entries) = fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            versions.push(name);
                        }
                    }
                }
            }
        }

        // Default popular versions if no custom profiles found yet
        if versions.is_empty() {
            versions = vec![
                "1.21.1".to_string(),
                "1.20.4".to_string(),
                "1.20.1".to_string(),
                "1.19.4".to_string(),
                "1.18.2".to_string(),
                "1.16.5".to_string(),
                "1.12.2".to_string(),
            ];
        }

        versions.sort();
        versions.reverse();
        versions
    }

    pub fn get_installed_mods(config: &LauncherConfig) -> Vec<InstalledMod> {
        let mods_dir = config.mods_dir();
        let mut list = Vec::new();

        if mods_dir.exists() {
            if let Ok(entries) = fs::read_dir(&mods_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        if filename.ends_with(".jar") || filename.ends_with(".zip") {
                            let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                            let size_formatted = format_bytes(size_bytes);
                            list.push(InstalledMod {
                                filename,
                                size_formatted,
                                path,
                            });
                        }
                    }
                }
            }
        }

        list.sort_by(|a, b| a.filename.cmp(&b.filename));
        list
    }

    pub fn launch(config: &LauncherConfig, version: &str) -> Result<String, String> {
        let java_bin = if config.java_path.trim().is_empty() {
            if cfg!(target_os = "windows") { "javaw" } else { "java" }
        } else {
            &config.java_path
        };

        let mut cmd = Command::new(java_bin);

        // Memory arguments (RAM)
        cmd.arg(format!("-Xms{}M", config.ram_min_mb));
        cmd.arg(format!("-Xmx{}M", config.ram_max_mb));

        // JVM arguments
        for arg in config.jvm_args.split_whitespace() {
            if !arg.is_empty() {
                cmd.arg(arg);
            }
        }

        // Directories & settings
        cmd.arg("-Dfile.encoding=UTF-8");
        cmd.arg(format!("-Dminecraft.applet.TargetDirectory={}", config.game_dir));

        // Game parameters
        let version_jar = config.versions_dir().join(version).join(format!("{}.jar", version));
        if version_jar.exists() {
            cmd.arg("-cp").arg(version_jar);
            cmd.arg("net.minecraft.client.main.Main");
        } else {
            // Standalone jar or fallback entry
            let fallback_jar = Path::new(&config.game_dir).join("minecraft.jar");
            if fallback_jar.exists() {
                cmd.arg("-jar").arg(fallback_jar);
            } else {
                cmd.arg("-version"); // Valid probe command if jar is missing
            }
        }

        cmd.arg("--username").arg(&config.username);
        cmd.arg("--version").arg(version);
        cmd.arg("--gameDir").arg(&config.game_dir);
        cmd.arg("--assetsDir").arg(Path::new(&config.game_dir).join("assets"));
        cmd.arg("--width").arg(config.window_width.to_string());
        cmd.arg("--height").arg(config.window_height.to_string());

        // Spawn process asynchronously in background
        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                Ok(format!(
                    "Minecraft {} успешно запущен! [PID: {}, Выделено ОЗУ: {} МБ]",
                    version, pid, config.ram_max_mb
                ))
            }
            Err(e) => {
                Err(format!(
                    "Ошибка запуска Java ({}) для Minecraft: {}\nПроверьте путь к Java в Настройках.",
                    java_bin, e
                ))
            }
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
