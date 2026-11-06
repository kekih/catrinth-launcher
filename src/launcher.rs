use crate::config::LauncherConfig;
use crate::java_manager::JavaManager;
use crate::mod_loaders::{maven_coord_to_path, ParsedVersion};
use crate::mojang_api::{
    extract_arg_values, is_allowed_by_rules, is_arg_allowed, offline_player_uuid, Library, VersionDetails,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

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
                            let json_path = entry.path().join(format!("{}.json", name));
                            if json_path.exists() {
                                versions.push(name);
                            }
                        }
                    }
                }
            }
        }

        versions.sort();
        versions.reverse();
        versions
    }

    pub fn is_version_installed(config: &LauncherConfig, version_str: &str) -> bool {
        let parsed = ParsedVersion::parse(version_str);
        let versions_dir = config.versions_dir();

        let candidate_names = [
            parsed.target_folder_name.clone(),
            version_str.to_string(),
            parsed.game_version.clone(),
        ];

        for name in &candidate_names {
            let v_dir = versions_dir.join(name);
            let json = v_dir.join(format!("{}.json", name));
            let jar = v_dir.join(format!("{}.jar", name));

            if json.exists() {
                if jar.exists() {
                    return true;
                }
                // Check if parent jar exists
                let parent_jar = versions_dir.join(&parsed.game_version).join(format!("{}.jar", parsed.game_version));
                if parent_jar.exists() {
                    return true;
                }
            }
        }

        // Fuzzy match for forge (e.g. "1.20.1-forge-47.3.0" for "1.20.1 (Forge)")
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains(&parsed.game_version) {
                    if (parsed.raw.contains("Forge") && (name.contains("forge") || name.contains("Forge")))
                        || (parsed.raw.contains("Fabric") && (name.contains("fabric") || name.contains("Fabric")))
                    {
                        let json = entry.path().join(format!("{}.json", name));
                        if json.exists() {
                            return true;
                        }
                    }
                }
            }
        }

        false
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

    pub fn launch(config: &LauncherConfig, version: &str) -> Result<(Child, PathBuf, String), String> {
        let parsed = ParsedVersion::parse(version);
        let versions_dir = config.versions_dir();

        // 1. Locate version folder & JSON file
        let mut target_dir = versions_dir.join(&parsed.target_folder_name);
        let mut target_json = target_dir.join(format!("{}.json", parsed.target_folder_name));
        let mut actual_id = parsed.target_folder_name.clone();

        if !target_json.exists() {
            let direct = versions_dir.join(version);
            let direct_json = direct.join(format!("{}.json", version));
            if direct_json.exists() {
                target_dir = direct;
                target_json = direct_json;
                actual_id = version.to_string();
            } else if let Ok(entries) = fs::read_dir(&versions_dir) {
                // Search for fuzzy match
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains(&parsed.game_version) {
                        if (parsed.raw.contains("Forge") && (name.contains("forge") || name.contains("Forge")))
                            || (parsed.raw.contains("Fabric") && (name.contains("fabric") || name.contains("Fabric")))
                        {
                            let candidate_json = entry.path().join(format!("{}.json", name));
                            if candidate_json.exists() {
                                target_dir = entry.path();
                                target_json = candidate_json;
                                actual_id = name;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if !target_json.exists() {
            // Fallback to game_version directory
            let gv_dir = versions_dir.join(&parsed.game_version);
            let gv_json = gv_dir.join(format!("{}.json", parsed.game_version));
            if gv_json.exists() {
                target_dir = gv_dir;
                target_json = gv_json;
                actual_id = parsed.game_version.clone();
            } else {
                return Err(format!(
                    "Версия {} не установлена! Нажмите кнопку «Скачать и запустить Minecraft».",
                    version
                ));
            }
        }

        // Parse target version details JSON
        let json_content = fs::read_to_string(&target_json)
            .map_err(|e| format!("Не удалось прочитать {}: {}", target_json.display(), e))?;
        let details: VersionDetails = serde_json::from_str(&json_content)
            .map_err(|e| format!("Ошибка парсинга версии {}: {}", actual_id, e))?;

        // Parent details if version inheritsFrom another version (like Fabric or Forge)
        let parent_details: Option<VersionDetails> = if let Some(ref parent_name) = details.inherits_from {
            let parent_json = versions_dir.join(parent_name).join(format!("{}.json", parent_name));
            if let Ok(content) = fs::read_to_string(&parent_json) {
                serde_json::from_str(&content).ok()
            } else {
                None
            }
        } else {
            None
        };

        // Determine client jar path
        let mut jar_path = target_dir.join(format!("{}.jar", actual_id));
        if !jar_path.exists() {
            if let Some(ref p) = details.inherits_from {
                let p_jar = versions_dir.join(p).join(format!("{}.jar", p));
                if p_jar.exists() {
                    jar_path = p_jar;
                }
            } else {
                let p_jar = versions_dir.join(&parsed.game_version).join(format!("{}.jar", parsed.game_version));
                if p_jar.exists() {
                    jar_path = p_jar;
                }
            }
        }

        // 2. Locate Java executable (prefers Eclipse Temurin)
        let java_bin = JavaManager::find_best_java_for_minecraft(&config.java_path, &parsed.game_version).ok_or_else(|| {
            format!(
                "Java не найдена на вашем компьютере!\n\
                Перейдите во вкладку «Java Runtimes» и нажмите «Установить Java 25» или «Java 21» в 1 клик."
            )
        })?;

        let mut cmd = Command::new(&java_bin);

        // Memory arguments (RAM)
        cmd.arg(format!("-Xms{}M", config.ram_min_mb));
        cmd.arg(format!("-Xmx{}M", config.ram_max_mb));

        // Custom user JVM args
        for arg in config.jvm_args.split_whitespace() {
            if !arg.is_empty() {
                cmd.arg(arg);
            }
        }

        // 3. Classpath construction
        let libraries_dir = config.libraries_dir();
        let mut cp_entries = Vec::new();

        let mut add_libs = |libs: &[Library]| {
            for lib in libs {
                if is_allowed_by_rules(lib.rules.as_deref()) {
                    let rel_path = if let Some(ref dls) = lib.downloads {
                        if let Some(ref art) = dls.artifact {
                            art.path.clone().unwrap_or_else(|| maven_coord_to_path(&lib.name))
                        } else {
                            maven_coord_to_path(&lib.name)
                        }
                    } else {
                        maven_coord_to_path(&lib.name)
                    };
                    let full_path = libraries_dir.join(&rel_path);
                    if full_path.exists() {
                        cp_entries.push(full_path);
                    }
                }
            }
        };

        // Add parent (vanilla) libraries first
        if let Some(ref p_det) = parent_details {
            if let Some(ref p_libs) = p_det.libraries {
                add_libs(p_libs);
            }
        }

        // Add loader/version libraries
        if let Some(ref libs) = details.libraries {
            add_libs(libs);
        }

        cp_entries.push(jar_path);

        let path_sep = if cfg!(target_os = "windows") { ";" } else { ":" };
        let classpath_str = cp_entries
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(path_sep);

        // 4. Substitution variables
        let assets_dir = config.assets_dir();
        let asset_index_id = details
            .asset_index
            .as_ref()
            .or_else(|| parent_details.as_ref().and_then(|p| p.asset_index.as_ref()))
            .map(|a| a.id.clone())
            .unwrap_or_else(|| parsed.game_version.clone());

        let natives_dir = target_dir.join("natives");
        let _ = fs::create_dir_all(&natives_dir);

        let offline_uuid = offline_player_uuid(&config.username);

        let subst = |input: &str| -> String {
            input
                .replace("${auth_player_name}", &config.username)
                .replace("${version_name}", &actual_id)
                .replace("${game_directory}", &config.game_dir)
                .replace("${assets_root}", &assets_dir.to_string_lossy())
                .replace("${assets_index_name}", &asset_index_id)
                .replace("${auth_uuid}", &offline_uuid)
                .replace("${auth_access_token}", "0")
                .replace("${clientid}", "0")
                .replace("${auth_xuid}", "0")
                .replace("${user_type}", "mojang")
                .replace("${version_type}", "release")
                .replace("${natives_directory}", &natives_dir.to_string_lossy())
                .replace("${launcher_name}", "Catrinth")
                .replace("${launcher_version}", "1.0.0")
                .replace("${classpath}", &classpath_str)
                .replace("${classpath_separator}", path_sep)
                .replace("${library_directory}", &libraries_dir.to_string_lossy())
        };

        // 5. JVM arguments evaluation
        let mut has_classpath_flag = false;

        let eval_jvm_args = |args: &[serde_json::Value], cmd: &mut Command, has_cp: &mut bool| {
            for arg_val in args {
                if is_arg_allowed(arg_val) {
                    for raw in extract_arg_values(arg_val) {
                        let replaced = subst(&raw);
                        if replaced == "-cp" || replaced == "-classpath" {
                            *has_cp = true;
                        }
                        cmd.arg(replaced);
                    }
                }
            }
        };

        if let Some(jvm_args) = parent_details.as_ref().and_then(|p| p.arguments.as_ref()).and_then(|a| a.jvm.as_ref()) {
            eval_jvm_args(jvm_args, &mut cmd, &mut has_classpath_flag);
        }

        if let Some(jvm_args) = details.arguments.as_ref().and_then(|a| a.jvm.as_ref()) {
            eval_jvm_args(jvm_args, &mut cmd, &mut has_classpath_flag);
        } else if parent_details.is_none() {
            cmd.arg(format!("-Djava.library.path={}", natives_dir.display()));
        }

        if !has_classpath_flag {
            cmd.arg("-cp").arg(&classpath_str);
        }

        cmd.arg("-Dfile.encoding=UTF-8");
        cmd.arg(format!(
            "-Dminecraft.applet.TargetDirectory={}",
            config.game_dir
        ));

        // 6. Main class
        cmd.arg(details.get_main_class());

        // 7. Game arguments
        let mut has_resolution = false;

        let eval_game_args = |args: &[serde_json::Value], cmd: &mut Command, has_res: &mut bool| {
            for arg_val in args {
                if is_arg_allowed(arg_val) {
                    for raw in extract_arg_values(arg_val) {
                        let replaced = subst(&raw);
                        if replaced == "--width" || replaced == "--height" {
                            *has_res = true;
                        }
                        cmd.arg(replaced);
                    }
                }
            }
        };

        if let Some(game_args) = details.arguments.as_ref().and_then(|a| a.game.as_ref()) {
            eval_game_args(game_args, &mut cmd, &mut has_resolution);
        } else if let Some(game_args) = parent_details.as_ref().and_then(|p| p.arguments.as_ref()).and_then(|a| a.game.as_ref()) {
            eval_game_args(game_args, &mut cmd, &mut has_resolution);
        } else if let Some(ref legacy_args) = details.minecraft_arguments {
            for token in legacy_args.split_whitespace() {
                let replaced = subst(token);
                if replaced == "--width" || replaced == "--height" {
                    has_resolution = true;
                }
                cmd.arg(replaced);
            }
        } else {
            cmd.arg("--username").arg(&config.username);
            cmd.arg("--version").arg(&actual_id);
            cmd.arg("--gameDir").arg(&config.game_dir);
            cmd.arg("--assetsDir").arg(&assets_dir);
            cmd.arg("--assetIndex").arg(&asset_index_id);
            cmd.arg("--uuid").arg(&offline_uuid);
            cmd.arg("--accessToken").arg("0");
            cmd.arg("--userType").arg("mojang");
        }

        if !has_resolution {
            cmd.arg("--width").arg(config.window_width.to_string());
            cmd.arg("--height").arg(config.window_height.to_string());
        }

        // Set working directory to game_dir
        let _ = fs::create_dir_all(&config.game_dir);
        cmd.current_dir(&config.game_dir);

        // Redirect stdout & stderr to game log file
        let log_dir = Path::new(&config.game_dir).join("logs");
        let _ = fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("catrinth-game.log");

        if let Ok(file_out) = fs::File::create(&log_path) {
            if let Ok(file_err) = file_out.try_clone() {
                cmd.stdout(std::process::Stdio::from(file_out));
                cmd.stderr(std::process::Stdio::from(file_err));
            }
        }

        // Spawn Minecraft process in background
        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                let msg = format!(
                    "Minecraft {} ({}) запущен! [PID: {}, Выделено ОЗУ: {} МБ]",
                    parsed.game_version, parsed.loader.label(), pid, config.ram_max_mb
                );
                Ok((child, log_path, msg))
            }
            Err(e) => Err(format!(
                "Ошибка запуска процесса Java ({}) для Minecraft: {}\nПроверьте путь к Java во вкладке «Java Runtimes».",
                java_bin.display(),
                e
            )),
        }
    }

    pub fn read_log_tail(path: &Path, max_lines: usize) -> String {
        read_log_tail(path, max_lines)
    }
}

pub fn read_log_tail(path: &Path, max_lines: usize) -> String {
    if let Ok(content) = fs::read_to_string(path) {
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() <= max_lines {
            return lines.join("\n");
        } else {
            return lines[lines.len() - max_lines..].join("\n");
        }
    }
    String::new()
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
