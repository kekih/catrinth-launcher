use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const RESOURCES_BASE_URL: &str = "https://resources.download.minecraft.net";
pub const LIBRARIES_BASE_URL: &str = "https://libraries.minecraft.net";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<ManifestVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub time: String,
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDetails {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndexRef>,
    pub downloads: Option<VersionDownloads>,
    pub libraries: Option<Vec<Library>>,
    pub arguments: Option<VersionArguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionInfo {
    pub component: Option<String>,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

impl VersionDetails {
    pub fn get_main_class(&self) -> String {
        self.main_class
            .clone()
            .unwrap_or_else(|| "net.minecraft.client.main.Main".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: Option<DownloadFile>,
    pub server: Option<DownloadFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFile {
    pub sha1: Option<String>,
    pub size: u64,
    pub url: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadFile>,
    pub classifiers: Option<HashMap<String, DownloadFile>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<RuleOs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleOs {
    pub name: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionArguments {
    pub game: Option<Vec<serde_json::Value>>,
    pub jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub stage: String,
    pub current: usize,
    pub total: usize,
    pub percent: f32,
    pub message: String,
    pub is_done: bool,
    pub error: Option<String>,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            stage: "Ожидание...".to_string(),
            current: 0,
            total: 0,
            percent: 0.0,
            message: String::new(),
            is_done: false,
            error: None,
        }
    }
}

pub fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn is_allowed_by_rules(rules: Option<&[Rule]>) -> bool {
    let rules = match rules {
        Some(r) if !r.is_empty() => r,
        _ => return true,
    };

    let os_name = current_os_name();
    let mut allowed = false;
    for rule in rules {
        let matches = match &rule.os {
            Some(rule_os) => rule_os.name.as_deref() == Some(os_name),
            None => true,
        };
        if matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

pub fn is_arg_allowed(val: &serde_json::Value) -> bool {
    if val.is_string() {
        return true;
    }
    if let Some(rules_val) = val.get("rules") {
        if let Ok(rules) = serde_json::from_value::<Vec<Rule>>(rules_val.clone()) {
            return is_allowed_by_rules(Some(&rules));
        }
    }
    true
}

pub fn extract_arg_values(val: &serde_json::Value) -> Vec<String> {
    if let Some(s) = val.as_str() {
        return vec![s.to_string()];
    }
    if let Some(val_field) = val.get("value") {
        if let Some(s) = val_field.as_str() {
            return vec![s.to_string()];
        }
        if let Some(arr) = val_field.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }
    Vec::new()
}

pub struct MojangClient {
    client: reqwest::blocking::Client,
}

impl MojangClient {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("CatrinthLauncher/1.0.0 (https://catrinth.ru)")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub fn fetch_manifest(&self, cache_dir: &Path) -> Result<VersionManifest, String> {
        let manifest_file = cache_dir.join("manifest.json");

        match self.client.get(VERSION_MANIFEST_URL).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(bytes) = resp.bytes() {
                    let _ = fs::create_dir_all(cache_dir);
                    let _ = fs::write(&manifest_file, &bytes);
                    if let Ok(manifest) = serde_json::from_slice::<VersionManifest>(&bytes) {
                        return Ok(manifest);
                    }
                }
            }
            _ => {}
        }

        // Fallback to local cache if offline
        if manifest_file.exists() {
            if let Ok(content) = fs::read_to_string(&manifest_file) {
                if let Ok(manifest) = serde_json::from_str::<VersionManifest>(&content) {
                    return Ok(manifest);
                }
            }
        }

        Err("Не удалось загрузить манифест версий Minecraft с piston-meta.mojang.com".to_string())
    }

    pub fn fetch_version_details(
        &self,
        game_dir: &Path,
        manifest: &VersionManifest,
        version_id: &str,
    ) -> Result<VersionDetails, String> {
        let version_dir = game_dir.join("versions").join(version_id);
        let version_json_file = version_dir.join(format!("{}.json", version_id));

        if version_json_file.exists() {
            if let Ok(content) = fs::read_to_string(&version_json_file) {
                if let Ok(details) = serde_json::from_str::<VersionDetails>(&content) {
                    return Ok(details);
                }
            }
        }

        let ver_entry = manifest
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .ok_or_else(|| format!("Версия {} не найдена в манифесте Mojang", version_id))?;

        let resp = self
            .client
            .get(&ver_entry.url)
            .send()
            .map_err(|e| format!("Ошибка загрузки деталей версии {}: {}", version_id, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "HTTP ошибка при загрузке версии {}: {}",
                version_id,
                resp.status()
            ));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| format!("Ошибка чтения данных версии: {}", e))?;

        let _ = fs::create_dir_all(&version_dir);
        let _ = fs::write(&version_json_file, &bytes);

        serde_json::from_slice::<VersionDetails>(&bytes)
            .map_err(|e| format!("Ошибка парсинга JSON версии {}: {}", version_id, e))
    }

    pub fn download_client_jar(
        &self,
        game_dir: &Path,
        details: &VersionDetails,
        progress: &Arc<Mutex<DownloadProgress>>,
    ) -> Result<PathBuf, String> {
        let client_download = details
            .downloads
            .as_ref()
            .and_then(|d| d.client.as_ref())
            .ok_or_else(|| format!("У версии {} нет клиентского .jar", details.id))?;

        let version_dir = game_dir.join("versions").join(&details.id);
        let jar_path = version_dir.join(format!("{}.jar", details.id));

        if jar_path.exists() {
            if let Ok(meta) = fs::metadata(&jar_path) {
                if meta.len() == client_download.size {
                    return Ok(jar_path);
                }
            }
        }

        {
            let mut p = progress.lock().unwrap();
            p.stage = format!("Скачивание Minecraft {}.jar...", details.id);
            p.percent = 0.15;
        }

        let _ = fs::create_dir_all(&version_dir);
        let part_path = version_dir.join(format!("{}.jar.part", details.id));

        let mut resp = self
            .client
            .get(&client_download.url)
            .send()
            .map_err(|e| format!("Не удалось скачать клиент Minecraft: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Ошибка скачивания client.jar: HTTP {}",
                resp.status()
            ));
        }

        let mut file = File::create(&part_path)
            .map_err(|e| format!("Не удалось создать файл {}: {}", part_path.display(), e))?;

        io::copy(&mut resp, &mut file)
            .map_err(|e| format!("Ошибка записи client.jar: {}", e))?;

        drop(file);
        let _ = fs::rename(&part_path, &jar_path);

        Ok(jar_path)
    }

    pub fn download_libraries(
        &self,
        game_dir: &Path,
        details: &VersionDetails,
        progress: &Arc<Mutex<DownloadProgress>>,
    ) -> Result<Vec<PathBuf>, String> {
        let libraries = match &details.libraries {
            Some(libs) => libs,
            None => return Ok(Vec::new()),
        };

        let libraries_dir = game_dir.join("libraries");
        let natives_dir = game_dir.join("versions").join(&details.id).join("natives");
        let _ = fs::create_dir_all(&libraries_dir);
        let _ = fs::create_dir_all(&natives_dir);

        let mut classpath_jars = Vec::new();
        let current_os = current_os_name();

        let allowed_libs: Vec<&Library> = libraries
            .iter()
            .filter(|lib| is_allowed_by_rules(lib.rules.as_deref()))
            .collect();

        let total = allowed_libs.len();

        for (idx, lib) in allowed_libs.iter().enumerate() {
            if let Some(ref dls) = lib.downloads {
                // 1. Artifact jar
                if let Some(ref art) = dls.artifact {
                    let rel_path = match &art.path {
                        Some(p) => p.clone(),
                        None => {
                            // Fallback derive from maven name
                            let parts: Vec<&str> = lib.name.split(':').collect();
                            if parts.len() >= 3 {
                                format!(
                                    "{}/{}/{}/{}-{}.jar",
                                    parts[0].replace('.', "/"),
                                    parts[1],
                                    parts[2],
                                    parts[1],
                                    parts[2]
                                )
                            } else {
                                format!("{}.jar", lib.name)
                            }
                        }
                    };

                    let dest = libraries_dir.join(&rel_path);
                    let need_download = if dest.exists() {
                        fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) != art.size
                    } else {
                        true
                    };

                    if need_download {
                        {
                            let mut p = progress.lock().unwrap();
                            p.stage = format!("Скачивание библиотек ({}/{})", idx + 1, total);
                            p.percent = 0.20 + 0.30 * ((idx + 1) as f32 / total as f32);
                        }

                        if let Some(parent) = dest.parent() {
                            let _ = fs::create_dir_all(parent);
                        }

                        let url = if art.url.is_empty() {
                            format!("{}/{}", LIBRARIES_BASE_URL, rel_path)
                        } else {
                            art.url.clone()
                        };

                        if let Ok(mut resp) = self.client.get(&url).send() {
                            if resp.status().is_success() {
                                let part = dest.with_extension("jar.part");
                                if let Ok(mut file) = File::create(&part) {
                                    if io::copy(&mut resp, &mut file).is_ok() {
                                        drop(file);
                                        let _ = fs::rename(&part, &dest);
                                    }
                                }
                            }
                        }
                    }

                    if dest.exists() {
                        classpath_jars.push(dest);
                    }
                }

                // 2. Classifiers / Natives (legacy 1.12 - 1.18)
                if let Some(ref classifiers) = dls.classifiers {
                    let native_key = match current_os {
                        "windows" => "natives-windows",
                        "osx" => "natives-osx",
                        _ => "natives-linux",
                    };

                    if let Some(native_file) = classifiers.get(native_key) {
                        let rel_path = match &native_file.path {
                            Some(p) => p.clone(),
                            None => format!("{}-{}.jar", lib.name, native_key),
                        };

                        let native_dest = libraries_dir.join(&rel_path);
                        if !native_dest.exists() {
                            if let Some(parent) = native_dest.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                            if let Ok(mut resp) = self.client.get(&native_file.url).send() {
                                if resp.status().is_success() {
                                    if let Ok(mut file) = File::create(&native_dest) {
                                        let _ = io::copy(&mut resp, &mut file);
                                    }
                                }
                            }
                        }

                        if native_dest.exists() {
                            let _ = extract_natives(&native_dest, &natives_dir);
                        }
                    }
                }
            }
        }

        Ok(classpath_jars)
    }

    pub fn download_assets(
        &self,
        game_dir: &Path,
        details: &VersionDetails,
        progress: &Arc<Mutex<DownloadProgress>>,
    ) -> Result<(), String> {
        let asset_index_ref = match &details.asset_index {
            Some(idx) => idx,
            None => return Ok(()),
        };

        let indexes_dir = game_dir.join("assets").join("indexes");
        let index_file = indexes_dir.join(format!("{}.json", asset_index_ref.id));
        let _ = fs::create_dir_all(&indexes_dir);

        {
            let mut p = progress.lock().unwrap();
            p.stage = format!("Загрузка индекса ресурсов {}.json...", asset_index_ref.id);
            p.percent = 0.55;
        }

        let asset_index: AssetIndex = if index_file.exists() {
            let content = fs::read_to_string(&index_file).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|_| {
                self.fetch_asset_index(&asset_index_ref.url, &index_file)
                    .unwrap_or(AssetIndex {
                        objects: HashMap::new(),
                    })
            })
        } else {
            self.fetch_asset_index(&asset_index_ref.url, &index_file)?
        };

        let objects_dir = game_dir.join("assets").join("objects");
        let _ = fs::create_dir_all(&objects_dir);

        // Find missing assets
        let mut missing_tasks: Vec<(String, PathBuf, u64)> = Vec::new();

        for (_key, obj) in asset_index.objects {
            if obj.hash.len() < 2 {
                continue;
            }
            let prefix = &obj.hash[..2];
            let dest_dir = objects_dir.join(prefix);
            let dest_file = dest_dir.join(&obj.hash);

            let need_dl = if dest_file.exists() {
                fs::metadata(&dest_file).map(|m| m.len()).unwrap_or(0) != obj.size
            } else {
                true
            };

            if need_dl {
                let url = format!("{}/{}/{}", RESOURCES_BASE_URL, prefix, obj.hash);
                missing_tasks.push((url, dest_file, obj.size));
            }
        }

        let total_missing = missing_tasks.len();
        if total_missing == 0 {
            let mut p = progress.lock().unwrap();
            p.percent = 0.95;
            p.stage = "Все ресурсы уже загружены и актуальны!".to_string();
            return Ok(());
        }

        // Multithreaded concurrent asset downloader pool (16 threads)
        let tasks_queue = Arc::new(Mutex::new(missing_tasks));
        let completed = Arc::new(AtomicUsize::new(0));
        let num_threads = 16.min(total_missing).max(1);
        let mut worker_handles = Vec::with_capacity(num_threads);

        let worker_client = Arc::new(
            reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("CatrinthLauncher/1.0.0")
                .build()
                .unwrap_or_default(),
        );

        for _ in 0..num_threads {
            let tasks = Arc::clone(&tasks_queue);
            let done = Arc::clone(&completed);
            let http = Arc::clone(&worker_client);

            worker_handles.push(thread::spawn(move || {
                loop {
                    let task = {
                        let mut guard = tasks.lock().unwrap();
                        guard.pop()
                    };

                    let (url, dest, _size) = match task {
                        Some(t) => t,
                        None => break,
                    };

                    if let Some(parent) = dest.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    if let Ok(resp) = http.get(&url).send() {
                        if resp.status().is_success() {
                            if let Ok(bytes) = resp.bytes() {
                                let _ = fs::write(&dest, &bytes);
                            }
                        }
                    }

                    done.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }

        // Monitor progress
        while completed.load(Ordering::Relaxed) < total_missing {
            let current = completed.load(Ordering::Relaxed);
            {
                let mut p = progress.lock().unwrap();
                p.current = current;
                p.total = total_missing;
                p.percent = 0.60 + 0.35 * (current as f32 / total_missing as f32);
                p.stage = format!(
                    "Загрузка текстур и звуков: {} / {} ({}%)",
                    current,
                    total_missing,
                    (p.percent * 100.0) as i32
                );
            }
            thread::sleep(Duration::from_millis(150));
        }

        for h in worker_handles {
            let _ = h.join();
        }

        Ok(())
    }

    fn fetch_asset_index(&self, url: &str, dest: &Path) -> Result<AssetIndex, String> {
        let resp = self
            .client
            .get(url)
            .send()
            .map_err(|e| format!("Ошибка загрузки индекса ресурсов: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Ошибка индекса ресурсов: HTTP {}", resp.status()));
        }

        let bytes = resp
            .bytes()
            .map_err(|e| format!("Ошибка чтения индекса ресурсов: {}", e))?;

        let _ = fs::write(dest, &bytes);
        serde_json::from_slice::<AssetIndex>(&bytes)
            .map_err(|e| format!("Ошибка парсинга индекса ресурсов: {}", e))
    }
}

pub fn extract_natives(jar_path: &Path, natives_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(natives_dir)?;
    let file = match File::open(jar_path) {
        Ok(f) => f,
        Err(_) => return Ok(()),
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let name = file.name().to_string();
        if name.starts_with("META-INF") || file.is_dir() {
            continue;
        }

        if !name.ends_with(".dll") && !name.ends_with(".so") && !name.ends_with(".dylib") {
            continue;
        }

        let outpath = match file.enclosed_name() {
            Some(path) => {
                let fname = path.file_name().unwrap_or(path.as_os_str());
                natives_dir.join(fname)
            }
            None => continue,
        };

        if let Ok(mut outfile) = File::create(&outpath) {
            let _ = io::copy(&mut file, &mut outfile);
        }
    }

    Ok(())
}

pub fn offline_player_uuid(username: &str) -> String {
    let mut bytes = [0u8; 16];
    let user_bytes = format!("OfflinePlayer:{}", username);
    for (i, b) in user_bytes.bytes().enumerate() {
        bytes[i % 16] ^= b;
        bytes[(i * 7 + 3) % 16] ^= b.rotate_left(3) ^ (i as u8);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11],
        bytes[12], bytes[13], bytes[14], bytes[15]
    )
}
