use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModHit {
    pub project_id: Option<String>,
    pub slug: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub downloads: Option<u64>,
    pub follows: Option<u64>,
    pub author: Option<String>,
    pub project_type: Option<String>,
    pub versions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub hits: Option<Vec<ModHit>>,
    pub total_hits: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: Option<bool>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModVersion {
    pub id: String,
    pub name: Option<String>,
    pub version_number: Option<String>,
    pub game_versions: Option<Vec<String>>,
    pub loaders: Option<Vec<String>>,
    pub files: Vec<VersionFile>,
}

pub struct CatrinthClient {
    client: reqwest::blocking::Client,
    base_url: String,
}

impl CatrinthClient {
    pub fn new(base_url: String) -> Self {
        let clean_url = base_url.trim_end_matches('/').to_string();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        Self {
            client,
            base_url: clean_url,
        }
    }

    pub fn check_health(&self) -> Result<String, String> {
        let url = format!("{}/health", self.base_url);
        let resp = self.client.get(&url).send()
            .map_err(|e| format!("Не удалось связаться с сервером Catrinth: {}", e))?;

        if resp.status().is_success() {
            Ok("Catrinth Hub в сети!".to_string())
        } else {
            Err(format!("Сервер вернул статус HTTP {}", resp.status()))
        }
    }

    pub fn search_mods(
        &self,
        query: &str,
        project_type: &str,
        limit: usize,
    ) -> Result<Vec<ModHit>, String> {
        let mut url = format!("{}/api/search?limit={}", self.base_url, limit);
        if !query.is_empty() {
            url.push_str(&format!("&query={}", urlencoding(query)));
        }
        if project_type != "all" {
            url.push_str(&format!("&project_type={}", project_type));
        }

        let resp = self.client.get(&url).send()
            .map_err(|e| format!("Ошибка поиска модов: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Ошибка API Catrinth: HTTP {}", resp.status()));
        }

        let search_res: SearchResponse = resp.json()
            .map_err(|e| format!("Ошибка разбора ответа поиска: {}", e))?;

        Ok(search_res.hits.unwrap_or_default())
    }

    pub fn get_project_versions(&self, slug_or_id: &str) -> Result<Vec<ModVersion>, String> {
        let url = format!("{}/v2/project/{}/version", self.base_url, slug_or_id);
        let resp = self.client.get(&url).send()
            .map_err(|e| format!("Ошибка получения версий: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("Ошибка API версий: HTTP {}", resp.status()));
        }

        let versions: Vec<ModVersion> = resp.json()
            .map_err(|e| format!("Ошибка парсинга версий: {}", e))?;

        Ok(versions)
    }

    pub fn download_file(
        &self,
        download_url: &str,
        target_path: &Path,
    ) -> Result<u64, String> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать папку: {}", e))?;
        }

        // Support relative or absolute URLs
        let full_url = if download_url.starts_with("http://") || download_url.starts_with("https://") {
            download_url.to_string()
        } else {
            format!("{}{}", self.base_url, download_url)
        };

        // Fast-track: resolve direct official CDN URL to bypass proxy/relay bottlenecks
        let direct_url = if let Some(idx) = full_url.find("/proxy/cdn/") {
            Some(format!("https://cdn.modrinth.com/{}", full_url[idx + "/proxy/cdn/".len()..].trim_start_matches('/')))
        } else if let Some(idx) = full_url.find("/cdn/") {
            Some(format!("https://cdn.modrinth.com/{}", full_url[idx + "/cdn/".len()..].trim_start_matches('/')))
        } else if let Some(idx) = full_url.find("/data/") {
            Some(format!("https://cdn.modrinth.com/data/{}", full_url[idx + "/data/".len()..].trim_start_matches('/')))
        } else {
            None
        };

        let mut target_resp = None;
        if let Some(ref direct) = direct_url {
            if let Ok(resp) = self.client.get(direct).send() {
                if resp.status().is_success() {
                    target_resp = Some(resp);
                }
            }
        }

        let mut resp = match target_resp {
            Some(r) => r,
            None => {
                let r = self.client.get(&full_url).send()
                    .map_err(|e| format!("Ошибка скачивания файла {}: {}", full_url, e))?;
                if !r.status().is_success() {
                    return Err(format!("Не удалось скачать: HTTP {}", r.status()));
                }
                r
            }
        };

        let mut file = File::create(target_path)
            .map_err(|e| format!("Ошибка создания файла: {}", e))?;

        let mut downloaded_bytes = 0u64;
        let mut buffer = [0u8; 262144]; // 256 KB chunk buffer
        loop {
            use std::io::Read;
            let read_bytes = resp.read(&mut buffer)
                .map_err(|e| format!("Ошибка чтения потока: {}", e))?;
            if read_bytes == 0 {
                break;
            }
            file.write_all(&buffer[..read_bytes])
                .map_err(|e| format!("Ошибка записи на диск: {}", e))?;
            downloaded_bytes += read_bytes as u64;
        }

        Ok(downloaded_bytes)
    }
}

fn urlencoding(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    encoded
}
