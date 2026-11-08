use crate::mojang_api::DownloadProgress;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModLoaderType {
    All,
    Fabric,
    NeoForge,
    Forge,
    Vanilla,
}

impl ModLoaderType {
    pub fn label(&self) -> &'static str {
        match self {
            ModLoaderType::All => "Все",
            ModLoaderType::Fabric => "Fabric",
            ModLoaderType::NeoForge => "NeoForge",
            ModLoaderType::Forge => "Forge",
            ModLoaderType::Vanilla => "Vanilla",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ModLoaderType::All => "Все",
            ModLoaderType::Fabric => "Fabric",
            ModLoaderType::NeoForge => "NeoForge",
            ModLoaderType::Forge => "Forge",
            ModLoaderType::Vanilla => "Vanilla",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedVersion {
    pub raw: String,
    pub game_version: String,
    pub loader: ModLoaderType,
    pub target_folder_name: String,
}

impl ParsedVersion {
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        if input.ends_with("(NeoForge)") {
            let base = input.trim_end_matches("(NeoForge)").trim();
            Self {
                raw: input.to_string(),
                game_version: base.to_string(),
                loader: ModLoaderType::NeoForge,
                target_folder_name: format!("neoforge-{}", base),
            }
        } else if input.ends_with("(Fabric)") {
            let base = input.trim_end_matches("(Fabric)").trim();
            Self {
                raw: input.to_string(),
                game_version: base.to_string(),
                loader: ModLoaderType::Fabric,
                target_folder_name: format!("fabric-{}", base),
            }
        } else if input.ends_with("(Forge)") {
            let base = input.trim_end_matches("(Forge)").trim();
            Self {
                raw: input.to_string(),
                game_version: base.to_string(),
                loader: ModLoaderType::Forge,
                target_folder_name: format!("forge-{}", base),
            }
        } else if input.ends_with("(Vanilla)") {
            let base = input.trim_end_matches("(Vanilla)").trim();
            Self {
                raw: input.to_string(),
                game_version: base.to_string(),
                loader: ModLoaderType::Vanilla,
                target_folder_name: base.to_string(),
            }
        } else if input.starts_with("neoforge-") || input.contains("neoforge") || input.contains("NeoForge") {
            let parts: Vec<&str> = input.split('-').collect();
            let gv = parts.last().unwrap_or(&input).to_string();
            Self {
                raw: input.to_string(),
                game_version: gv,
                loader: ModLoaderType::NeoForge,
                target_folder_name: input.to_string(),
            }
        } else if input.starts_with("fabric-loader-") || input.contains("fabric") || input.contains("Fabric") {
            let parts: Vec<&str> = input.split('-').collect();
            let gv = parts.last().unwrap_or(&input).to_string();
            Self {
                raw: input.to_string(),
                game_version: gv,
                loader: ModLoaderType::Fabric,
                target_folder_name: input.to_string(),
            }
        } else if input.contains("forge") || input.contains("Forge") {
            let parts: Vec<&str> = input.split('-').collect();
            let gv = parts.first().unwrap_or(&input).to_string();
            Self {
                raw: input.to_string(),
                game_version: gv,
                loader: ModLoaderType::Forge,
                target_folder_name: input.to_string(),
            }
        } else {
            Self {
                raw: input.to_string(),
                game_version: input.to_string(),
                loader: ModLoaderType::Vanilla,
                target_folder_name: input.to_string(),
            }
        }
    }
}

pub fn is_fabric_supported(game_version: &str) -> bool {
    let parts: Vec<&str> = game_version.split('.').collect();
    if parts.len() >= 2 {
        if let (Ok(maj), Ok(min)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            if maj == 1 && min >= 14 || maj > 1 {
                return true;
            }
        }
    }
    game_version.starts_with("1.14")
        || game_version.starts_with("1.15")
        || game_version.starts_with("1.16")
        || game_version.starts_with("1.17")
        || game_version.starts_with("1.18")
        || game_version.starts_with("1.19")
        || game_version.starts_with("1.20")
        || game_version.starts_with("1.21")
        || game_version.starts_with("1.22")
}

pub fn is_neoforge_supported(game_version: &str) -> bool {
    get_neoforge_recommended_version(game_version).is_some()
}

pub fn is_forge_supported(game_version: &str) -> bool {
    get_forge_recommended_version(game_version).is_some()
}

pub fn get_default_versions() -> Vec<String> {
    vec![
        // 1.21.x
        "1.21.4 (Fabric)".to_string(),
        "1.21.4 (NeoForge)".to_string(),
        "1.21.4 (Vanilla)".to_string(),
        "1.21.3 (Fabric)".to_string(),
        "1.21.3 (NeoForge)".to_string(),
        "1.21.3 (Vanilla)".to_string(),
        "1.21.1 (Fabric)".to_string(),
        "1.21.1 (NeoForge)".to_string(),
        "1.21.1 (Vanilla)".to_string(),
        "1.21 (Fabric)".to_string(),
        "1.21 (NeoForge)".to_string(),
        "1.21 (Vanilla)".to_string(),

        // 1.20.x
        "1.20.6 (Fabric)".to_string(),
        "1.20.6 (NeoForge)".to_string(),
        "1.20.6 (Vanilla)".to_string(),
        "1.20.4 (Fabric)".to_string(),
        "1.20.4 (NeoForge)".to_string(),
        "1.20.4 (Forge)".to_string(),
        "1.20.4 (Vanilla)".to_string(),
        "1.20.2 (Fabric)".to_string(),
        "1.20.2 (NeoForge)".to_string(),
        "1.20.2 (Forge)".to_string(),
        "1.20.2 (Vanilla)".to_string(),
        "1.20.1 (Fabric)".to_string(),
        "1.20.1 (Forge)".to_string(),
        "1.20.1 (Vanilla)".to_string(),
        "1.20 (Fabric)".to_string(),
        "1.20 (Forge)".to_string(),
        "1.20 (Vanilla)".to_string(),

        // 1.19.x
        "1.19.4 (Fabric)".to_string(),
        "1.19.4 (Forge)".to_string(),
        "1.19.4 (Vanilla)".to_string(),
        "1.19.3 (Fabric)".to_string(),
        "1.19.3 (Forge)".to_string(),
        "1.19.3 (Vanilla)".to_string(),
        "1.19.2 (Fabric)".to_string(),
        "1.19.2 (Forge)".to_string(),
        "1.19.2 (Vanilla)".to_string(),
        "1.19 (Fabric)".to_string(),
        "1.19 (Forge)".to_string(),
        "1.19 (Vanilla)".to_string(),

        // 1.18.x
        "1.18.2 (Fabric)".to_string(),
        "1.18.2 (Forge)".to_string(),
        "1.18.2 (Vanilla)".to_string(),
        "1.18.1 (Fabric)".to_string(),
        "1.18.1 (Forge)".to_string(),
        "1.18.1 (Vanilla)".to_string(),

        // 1.17.x
        "1.17.1 (Fabric)".to_string(),
        "1.17.1 (Forge)".to_string(),
        "1.17.1 (Vanilla)".to_string(),

        // 1.16.x
        "1.16.5 (Fabric)".to_string(),
        "1.16.5 (Forge)".to_string(),
        "1.16.5 (Vanilla)".to_string(),
        "1.16.4 (Fabric)".to_string(),
        "1.16.4 (Forge)".to_string(),
        "1.16.4 (Vanilla)".to_string(),

        // 1.15.x
        "1.15.2 (Fabric)".to_string(),
        "1.15.2 (Forge)".to_string(),
        "1.15.2 (Vanilla)".to_string(),

        // 1.14.x
        "1.14.4 (Fabric)".to_string(),
        "1.14.4 (Forge)".to_string(),
        "1.14.4 (Vanilla)".to_string(),

        // 1.12.x
        "1.12.2 (Forge)".to_string(),
        "1.12.2 (Vanilla)".to_string(),

        // Legacy popular
        "1.8.9 (Forge)".to_string(),
        "1.8.9 (Vanilla)".to_string(),
        "1.7.10 (Forge)".to_string(),
        "1.7.10 (Vanilla)".to_string(),
    ]
}

// Convert Maven artifact coordinates "group:artifact:version" to file path
pub fn maven_coord_to_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() >= 3 {
        let group_path = parts[0].replace('.', "/");
        let artifact = parts[1];
        let version = parts[2];
        let classifier = if parts.len() >= 4 {
            format!("-{}", parts[3])
        } else {
            String::new()
        };
        format!(
            "{}/{}/{}/{}-{}{}.jar",
            group_path, artifact, version, artifact, version, classifier
        )
    } else {
        format!("{}.jar", name)
    }
}

#[derive(Debug, Deserialize)]
struct FabricLoaderMeta {
    loader: FabricLoaderInfo,
}

#[derive(Debug, Deserialize)]
struct FabricLoaderInfo {
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FabricProfileJson {
    id: String,
    #[serde(rename = "inheritsFrom")]
    inherits_from: String,
    #[serde(rename = "mainClass")]
    main_class: String,
    #[serde(default)]
    arguments: Option<serde_json::Value>,
    #[serde(default)]
    libraries: Vec<FabricLibraryEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FabricLibraryEntry {
    name: String,
    url: Option<String>,
}

// Install Fabric loader profile and libraries
pub fn install_fabric(
    game_version: &str,
    game_dir: &str,
    progress: Arc<Mutex<DownloadProgress>>,
) -> Result<String, String> {
    let versions_dir = Path::new(game_dir).join("versions");
    let libraries_dir = Path::new(game_dir).join("libraries");
    let target_name = format!("fabric-{}", game_version);
    let target_dir = versions_dir.join(&target_name);
    let target_json = target_dir.join(format!("{}.json", target_name));

    let _ = fs::create_dir_all(&target_dir);
    let _ = fs::create_dir_all(&libraries_dir);

    {
        let mut p = progress.lock().unwrap();
        p.stage = format!("Поиск актуальной версии Fabric Loader для Minecraft {}...", game_version);
        p.percent = 0.10;
        p.is_done = false;
        p.error = None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .unwrap_or_default();

    // 1. Get available Fabric loader versions for this game version
    let meta_url = format!("https://meta.fabricmc.net/v2/versions/loader/{}", game_version);
    let loaders: Vec<FabricLoaderMeta> = client
        .get(&meta_url)
        .send()
        .map_err(|e| format!("Не удалось связаться с сервером Fabric: {}", e))?
        .json()
        .map_err(|e| format!("Ошибка парсинга списка версий Fabric: {}", e))?;

    let latest_loader = loaders
        .first()
        .map(|l| l.loader.version.clone())
        .unwrap_or_else(|| "0.16.10".to_string());

    {
        let mut p = progress.lock().unwrap();
        p.stage = format!("Загрузка профиля Fabric Loader {} для {}...", latest_loader, game_version);
        p.percent = 0.25;
    }

    // 2. Fetch full profile JSON
    let profile_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        game_version, latest_loader
    );

    let profile_text = client
        .get(&profile_url)
        .send()
        .map_err(|e| format!("Ошибка загрузки профиля Fabric: {}", e))?
        .text()
        .map_err(|e| format!("Ошибка чтения ответа профиля Fabric: {}", e))?;

    let mut profile: FabricProfileJson = serde_json::from_str(&profile_text)
        .map_err(|e| format!("Ошибка структуры JSON профиля Fabric: {}", e))?;

    profile.id = target_name.clone();
    profile.inherits_from = game_version.to_string();

    let updated_json = serde_json::to_string_pretty(&profile)
        .map_err(|e| format!("Ошибка сериализации профиля Fabric: {}", e))?;
    fs::write(&target_json, updated_json)
        .map_err(|e| format!("Ошибка сохранения {}: {}", target_json.display(), e))?;

    // 3. Download Fabric libraries
    let total_libs = profile.libraries.len().max(1);
    for (i, lib) in profile.libraries.iter().enumerate() {
        let rel_path = maven_coord_to_path(&lib.name);
        let lib_dest = libraries_dir.join(&rel_path);

        if !lib_dest.exists() {
            if let Some(parent) = lib_dest.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let base_url = lib
                .url
                .as_deref()
                .unwrap_or("https://maven.fabricmc.net/");
            let full_url = format!("{}{}", base_url, rel_path);

            {
                let mut p = progress.lock().unwrap();
                let pct = 0.30 + (i as f32 / total_libs as f32) * 0.65;
                p.percent = pct;
                p.stage = format!(
                    "Скачивание библиотек Fabric ({}/{}): {}",
                    i + 1,
                    total_libs,
                    lib.name.split(':').nth(1).unwrap_or(&lib.name)
                );
            }

            if let Ok(mut resp) = client.get(&full_url).send() {
                if resp.status().is_success() {
                    if let Ok(mut out) = File::create(&lib_dest) {
                        let _ = std::io::copy(&mut resp, &mut out);
                    }
                }
            }
        }
    }

    {
        let mut p = progress.lock().unwrap();
        p.stage = format!("Fabric Loader для Minecraft {} успешно установлен!", game_version);
        p.percent = 1.0;
        p.is_done = true;
    }

    Ok(target_name)
}

// Recommended NeoForge version mapping
pub fn get_neoforge_recommended_version(game_version: &str) -> Option<&'static str> {
    match game_version {
        "1.21.4" => Some("21.4.58-beta"),
        "1.21.3" => Some("21.3.56-beta"),
        "1.21.2" => Some("21.2.1-beta"),
        "1.21.1" => Some("21.1.115"),
        "1.21"   => Some("21.0.167"),
        "1.20.6" => Some("20.6.119"),
        "1.20.5" => Some("20.5.21-beta"),
        "1.20.4" => Some("20.4.237"),
        "1.20.3" => Some("20.3.8-beta"),
        "1.20.2" => Some("20.2.88"),
        _ => None,
    }
}

// Download and run NeoForge installer in headless mode
pub fn install_neoforge(
    game_version: &str,
    game_dir: &str,
    java_bin: &Path,
    progress: Arc<Mutex<DownloadProgress>>,
) -> Result<String, String> {
    let neoforge_ver = get_neoforge_recommended_version(game_version).ok_or_else(|| {
        format!(
            "Для версии Minecraft {} загрузчик NeoForge не найден. Поддерживаются версии 1.20.2 - 1.21.x!",
            game_version
        )
    })?;

    let installer_url = format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{}/neoforge-{}-installer.jar",
        neoforge_ver, neoforge_ver
    );

    let temp_installer = Path::new(game_dir).join(format!("neoforge-{}-installer.jar", neoforge_ver));

    {
        let mut p = progress.lock().unwrap();
        p.stage = format!("Скачивание установщика NeoForge {}...", neoforge_ver);
        p.percent = 0.15;
        p.is_done = false;
        p.error = None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_default();

    let mut resp = client
        .get(&installer_url)
        .send()
        .map_err(|e| format!("Не удалось скачать установщик NeoForge: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Ошибка загрузки установщика NeoForge: HTTP {}", resp.status()));
    }

    let total_size = resp.content_length().unwrap_or(8 * 1024 * 1024);
    let mut file = File::create(&temp_installer)
        .map_err(|e| format!("Ошибка создания файла установщика NeoForge: {}", e))?;

    let mut downloaded = 0u64;
    let mut buffer = [0u8; 32768];

    loop {
        let read = resp
            .read(&mut buffer)
            .map_err(|e| format!("Ошибка скачивания NeoForge: {}", e))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("Ошибка записи установщика NeoForge: {}", e))?;
        downloaded += read as u64;

        let frac = (downloaded as f32 / total_size as f32).clamp(0.0, 1.0);
        if downloaded % (256 * 1024) < 32768 {
            let mut p = progress.lock().unwrap();
            p.percent = 0.15 + 0.50 * frac;
            p.stage = format!(
                "Загрузка установщика NeoForge: {:.1} / {:.1} МБ",
                downloaded as f64 / 1_048_576.0,
                total_size as f64 / 1_048_576.0
            );
        }
    }

    drop(file);

    {
        let mut p = progress.lock().unwrap();
        p.stage = format!("Установка библиотек и патчей NeoForge {} (это может занять около 30 секунд)...", neoforge_ver);
        p.percent = 0.70;
    }

    let status = Command::new(java_bin)
        .arg("-jar")
        .arg(&temp_installer)
        .arg("--installClient")
        .arg(game_dir)
        .current_dir(game_dir)
        .status();

    let _ = fs::remove_file(&temp_installer);

    match status {
        Ok(st) if st.success() => {
            let mut p = progress.lock().unwrap();
            p.stage = format!("NeoForge {} успешно установлен!", neoforge_ver);
            p.percent = 1.0;
            p.is_done = true;

            let versions_dir = Path::new(game_dir).join("versions");
            if let Ok(entries) = fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("neoforge") || name.contains("NeoForge") {
                        if name.contains(game_version) {
                            return Ok(name);
                        }
                    }
                }
            }
            Ok(format!("neoforge-{}", game_version))
        }
        Ok(st) => Err(format!("Установщик NeoForge завершился с кодом ошибки: {}", st)),
        Err(e) => Err(format!("Не удалось запустить установщик NeoForge: {}", e)),
    }
}

// Recommended Forge version mapping
pub fn get_forge_recommended_version(game_version: &str) -> Option<&'static str> {
    match game_version {
        "1.20.4" => Some("49.0.38"),
        "1.20.3" => Some("48.1.0"),
        "1.20.2" => Some("48.1.0"),
        "1.20.1" => Some("47.3.0"),
        "1.20"   => Some("46.0.14"),
        "1.19.4" => Some("45.2.0"),
        "1.19.3" => Some("44.1.0"),
        "1.19.2" => Some("43.3.0"),
        "1.19.1" => Some("42.1.0"),
        "1.19"   => Some("41.1.0"),
        "1.18.2" => Some("40.2.0"),
        "1.18.1" => Some("39.1.0"),
        "1.18"   => Some("38.0.17"),
        "1.17.1" => Some("37.1.1"),
        "1.16.5" => Some("36.2.39"),
        "1.16.4" => Some("35.1.37"),
        "1.16.3" => Some("34.1.42"),
        "1.16.2" => Some("33.0.61"),
        "1.16.1" => Some("32.0.108"),
        "1.15.2" => Some("31.2.57"),
        "1.15.1" => Some("30.0.51"),
        "1.15"   => Some("29.0.4"),
        "1.14.4" => Some("28.2.26"),
        "1.14.3" => Some("27.0.60"),
        "1.14.2" => Some("26.0.63"),
        "1.13.2" => Some("25.0.223"),
        "1.12.2" => Some("14.23.5.2859"),
        "1.12.1" => Some("14.22.1.2478"),
        "1.12"   => Some("14.21.1.2385"),
        "1.11.2" => Some("13.20.1.2588"),
        "1.10.2" => Some("12.18.3.2511"),
        "1.9.4"  => Some("12.16.1.1907"),
        "1.8.9"  => Some("11.15.1.2318"),
        "1.7.10" => Some("10.13.4.1614"),
        _ => None,
    }
}

// Download and run Forge installer in headless mode
pub fn install_forge(
    game_version: &str,
    game_dir: &str,
    java_bin: &Path,
    progress: Arc<Mutex<DownloadProgress>>,
) -> Result<String, String> {
    let forge_ver = get_forge_recommended_version(game_version).ok_or_else(|| {
        format!(
            "Для версии Minecraft {} загрузчик Forge не найден. Рекомендуем использовать Fabric!",
            game_version
        )
    })?;

    let installer_url = format!(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/{}-{}/forge-{}-{}-installer.jar",
        game_version, forge_ver, game_version, forge_ver
    );

    let temp_installer = Path::new(game_dir).join(format!("forge-{}-{}-installer.jar", game_version, forge_ver));

    {
        let mut p = progress.lock().unwrap();
        p.stage = format!("Скачивание установщика Minecraft Forge {}...", forge_ver);
        p.percent = 0.15;
        p.is_done = false;
        p.error = None;
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_default();

    let mut resp = client
        .get(&installer_url)
        .send()
        .map_err(|e| format!("Не удалось скачать установщик Forge: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Ошибка загрузки установщика Forge: HTTP {}", resp.status()));
    }

    let total_size = resp.content_length().unwrap_or(6 * 1024 * 1024);
    let mut file = File::create(&temp_installer)
        .map_err(|e| format!("Ошибка создания файла установщика Forge: {}", e))?;

    let mut downloaded = 0u64;
    let mut buffer = [0u8; 32768];

    loop {
        let read = resp
            .read(&mut buffer)
            .map_err(|e| format!("Ошибка скачивания Forge: {}", e))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("Ошибка записи установщика Forge: {}", e))?;
        downloaded += read as u64;

        let frac = (downloaded as f32 / total_size as f32).clamp(0.0, 1.0);
        if downloaded % (256 * 1024) < 32768 {
            let mut p = progress.lock().unwrap();
            p.percent = 0.15 + 0.50 * frac;
            p.stage = format!(
                "Загрузка установщика Forge: {:.1} / {:.1} МБ",
                downloaded as f64 / 1_048_576.0,
                total_size as f64 / 1_048_576.0
            );
        }
    }

    drop(file);

    {
        let mut p = progress.lock().unwrap();
        p.stage = format!("Установка библиотек и патчей Forge {} (это может занять около 30 секунд)...", forge_ver);
        p.percent = 0.70;
    }

    let status = Command::new(java_bin)
        .arg("-jar")
        .arg(&temp_installer)
        .arg("--installClient")
        .arg(game_dir)
        .current_dir(game_dir)
        .status();

    let _ = fs::remove_file(&temp_installer);

    match status {
        Ok(st) if st.success() => {
            let mut p = progress.lock().unwrap();
            p.stage = format!("Minecraft Forge {} успешно установлен!", forge_ver);
            p.percent = 1.0;
            p.is_done = true;

            let versions_dir = Path::new(game_dir).join("versions");
            if let Ok(entries) = fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("forge") || name.contains("Forge") {
                        if name.contains(game_version) {
                            return Ok(name);
                        }
                    }
                }
            }
            Ok(format!("{}-forge-{}", game_version, forge_ver))
        }
        Ok(st) => Err(format!("Установщик Forge завершился с кодом ошибки: {}", st)),
        Err(e) => Err(format!("Не удалось запустить установщик Forge: {}", e)),
    }
}
