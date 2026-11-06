use crate::mojang_api::DownloadProgress;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TemurinVersionInfo {
    pub version: u32,
    pub title: &'static str,
    pub tag: &'static str,
    pub description: &'static str,
}

pub const ALL_TEMURIN_VERSIONS: &[TemurinVersionInfo] = &[
    TemurinVersionInfo {
        version: 25,
        title: "Eclipse Temurin 25 (JDK / Next-Gen)",
        tag: "LATEST / NEXT-GEN",
        description: "Новейшая версия OpenJDK 25. Подходит для Minecraft 1.21+ и Fabric. Новейшие оптимизации ZGC, генеративный JIT.",
    },
    TemurinVersionInfo {
        version: 24,
        title: "Eclipse Temurin 24",
        tag: "FEATURE",
        description: "Современная версия Java 24 для Minecraft 1.21+ с обновленными стандартами JVM.",
    },
    TemurinVersionInfo {
        version: 23,
        title: "Eclipse Temurin 23",
        tag: "FEATURE",
        description: "Java 23 с расширенным ZGC и высокой скоростью отклика.",
    },
    TemurinVersionInfo {
        version: 22,
        title: "Eclipse Temurin 22",
        tag: "FEATURE",
        description: "Стабильный релиз Java 22.",
    },
    TemurinVersionInfo {
        version: 21,
        title: "Eclipse Temurin 21 (LTS)",
        tag: "РЕКОМЕНДУЕТСЯ (LTS)",
        description: "Официальный стандарт для Minecraft 1.20.5+ и 1.21+. Долгосрочная поддержка (LTS), максимальная стабильность сборщика мусора G1GC.",
    },
    TemurinVersionInfo {
        version: 17,
        title: "Eclipse Temurin 17 (LTS)",
        tag: "LTS",
        description: "Необходима для Minecraft версий 1.17 – 1.20.4, а также Forge и Fabric для этих версий.",
    },
    TemurinVersionInfo {
        version: 11,
        title: "Eclipse Temurin 11 (LTS)",
        tag: "LTS",
        description: "Долгосрочный релиз Java 11 для промежуточных сборок и серверных плагинов.",
    },
    TemurinVersionInfo {
        version: 8,
        title: "Eclipse Temurin 8 (LTS)",
        tag: "LTS / КЛАССИКА",
        description: "Необходима для классических версий Minecraft 1.12.2, 1.7.10 и старых сборок Forge.",
    },
];

pub struct JavaManager;

impl JavaManager {
    pub fn get_runtime_base_dir() -> PathBuf {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        exe_dir.join("runtime")
    }

    pub fn get_temurin_dir(major_version: u32) -> PathBuf {
        Self::get_runtime_base_dir().join(format!("temurin-{}", major_version))
    }

    pub fn get_temurin_java_bin(major_version: u32) -> Option<PathBuf> {
        let dir = Self::get_temurin_dir(major_version);
        if !dir.exists() {
            return None;
        }
        find_java_in_dir(&dir)
    }

    pub fn is_temurin_installed(major_version: u32) -> bool {
        Self::get_temurin_java_bin(major_version).is_some()
    }

    pub fn find_best_java_for_minecraft(configured_path: &str, mc_version: &str) -> Option<PathBuf> {
        // 1. If user explicitly configured a valid Java path
        let trimmed = configured_path.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.exists() {
                return Some(p);
            }
        }

        // Determine candidate Java versions based on Minecraft version:
        // Minecraft 1.20.5+ / 1.21+ -> Java 21, 25, 24, 23, 22
        // Minecraft 1.17 - 1.20.4 -> Java 17, 21, 25
        // Minecraft <= 1.16.5 -> Java 8, 17
        let candidates: &[u32] = if mc_version.starts_with("1.21") || mc_version.starts_with("1.20.5") || mc_version.starts_with("1.20.6") {
            &[21, 25, 24, 23, 22]
        } else if mc_version.starts_with("1.20") || mc_version.starts_with("1.19") || mc_version.starts_with("1.18") || mc_version.starts_with("1.17") {
            &[17, 21, 25]
        } else {
            &[8, 17, 21]
        };

        // Check local Temurin runtime in candidate order
        for &ver in candidates {
            if let Some(bin) = Self::get_temurin_java_bin(ver) {
                return Some(bin);
            }
        }

        // Check any installed Temurin version as fallback
        for info in ALL_TEMURIN_VERSIONS {
            if let Some(bin) = Self::get_temurin_java_bin(info.version) {
                return Some(bin);
            }
        }

        // Check system PATH
        let probe = if cfg!(target_os = "windows") { "javaw.exe" } else { "java" };
        if let Ok(output) = Command::new(probe).arg("-version").output() {
            if output.status.success() || !output.stderr.is_empty() {
                return Some(PathBuf::from(probe));
            }
        }

        if cfg!(target_os = "windows") {
            if let Ok(output) = Command::new("java.exe").arg("-version").output() {
                if output.status.success() || !output.stderr.is_empty() {
                    return Some(PathBuf::from("java.exe"));
                }
            }

            // Search standard Windows Java directories
            let roots = [
                r"C:\Program Files\Eclipse Adoptium",
                r"C:\Program Files\Java",
                r"C:\Program Files\BellSoft",
                r"C:\Program Files\Microsoft",
                r"C:\Program Files\Zulu",
                r"C:\Program Files\Amazon Corretto",
            ];

            for root in roots {
                let path = Path::new(root);
                if path.exists() {
                    if let Ok(entries) = fs::read_dir(path) {
                        for entry in entries.flatten() {
                            if let Some(bin) = find_java_in_dir(&entry.path()) {
                                return Some(bin);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub fn download_and_install_temurin(
        major_version: u32,
        progress: Arc<Mutex<DownloadProgress>>,
    ) -> Result<PathBuf, String> {
        let os_slug = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "mac"
        } else {
            "linux"
        };

        let target_dir = Self::get_temurin_dir(major_version);
        let _ = fs::create_dir_all(&target_dir);

        let is_zip = cfg!(target_os = "windows");
        let archive_path = target_dir.join(if is_zip { "temurin.zip" } else { "temurin.tar.gz" });

        {
            let mut p = progress.lock().unwrap();
            p.stage = format!("Подключение к серверам Eclipse Temurin (Java {})...", major_version);
            p.percent = 0.05;
            p.is_done = false;
            p.error = None;
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .unwrap_or_default();

        // 1. Try JRE first
        let jre_url = format!(
            "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/x64/jre/hotspot/normal/eclipse",
            major_version, os_slug
        );
        let jdk_url = format!(
            "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/x64/jdk/hotspot/normal/eclipse",
            major_version, os_slug
        );

        let mut resp = match client.get(&jre_url).send() {
            Ok(r) if r.status().is_success() => r,
            _ => {
                // Fallback to JDK
                client.get(&jdk_url).send().map_err(|e| {
                    format!("Не удалось скачать Eclipse Temurin Java {}: {}", major_version, e)
                })?
            }
        };

        if !resp.status().is_success() {
            return Err(format!(
                "Ошибка серверов Adoptium: HTTP {} при загрузке Java {}",
                resp.status(),
                major_version
            ));
        }

        let total_size = resp.content_length().unwrap_or(48 * 1024 * 1024);

        let mut file = File::create(&archive_path)
            .map_err(|e| format!("Ошибка создания файла архива Java: {}", e))?;

        let mut downloaded_bytes = 0u64;
        let mut buffer = [0u8; 32768];

        loop {
            let read_bytes = resp
                .read(&mut buffer)
                .map_err(|e| format!("Ошибка чтения потока загрузки Java: {}", e))?;
            if read_bytes == 0 {
                break;
            }
            file.write_all(&buffer[..read_bytes])
                .map_err(|e| format!("Ошибка записи архива Java на диск: {}", e))?;
            downloaded_bytes += read_bytes as u64;

            let frac = (downloaded_bytes as f32 / total_size as f32).clamp(0.0, 1.0);
            let mb_done = downloaded_bytes as f64 / (1024.0 * 1024.0);
            let mb_total = total_size as f64 / (1024.0 * 1024.0);

            if downloaded_bytes % (256 * 1024) < 32768 {
                let mut p = progress.lock().unwrap();
                p.percent = 0.05 + 0.80 * frac;
                p.stage = format!(
                    "Скачивание Eclipse Temurin Java {}: {:.1} / {:.1} МБ ({}%)",
                    major_version,
                    mb_done,
                    mb_total,
                    (p.percent * 100.0) as i32
                );
            }
        }

        drop(file);

        {
            let mut p = progress.lock().unwrap();
            p.stage = format!("Распаковка Eclipse Temurin Java {}...", major_version);
            p.percent = 0.88;
        }

        // Unpack archive
        if is_zip {
            let archive_file = File::open(&archive_path)
                .map_err(|e| format!("Не удалось открыть скачанный архив Java: {}", e))?;
            let mut archive = zip::ZipArchive::new(archive_file)
                .map_err(|e| format!("Ошибка формата zip архива Java: {}", e))?;

            for i in 0..archive.len() {
                let mut item = archive.by_index(i)
                    .map_err(|e| format!("Ошибка чтения элемента архива: {}", e))?;
                let raw_path = match item.enclosed_name() {
                    Some(p) => p.to_path_buf(),
                    None => continue,
                };

                // Strip top-level directory (e.g. jdk-25.0.5+8/)
                let components: Vec<_> = raw_path.components().collect();
                let rel_path = if components.len() > 1 {
                    components[1..].iter().collect::<PathBuf>()
                } else {
                    raw_path
                };

                let outpath = target_dir.join(rel_path);
                if item.is_dir() {
                    let _ = fs::create_dir_all(&outpath);
                } else {
                    if let Some(parent) = outpath.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Ok(mut outfile) = File::create(&outpath) {
                        let _ = io::copy(&mut item, &mut outfile);
                    }
                }
            }
        } else {
            // Linux / macOS: use tar command
            let status = Command::new("tar")
                .arg("-xzf")
                .arg(&archive_path)
                .arg("--strip-components=1")
                .arg("-C")
                .arg(&target_dir)
                .status();

            if let Err(e) = status {
                return Err(format!("Не удалось распаковать архив через tar: {}", e));
            }
        }

        // Clean up temporary archive file
        let _ = fs::remove_file(&archive_path);

        // Find the installed binary
        let java_bin = find_java_in_dir(&target_dir).ok_or_else(|| {
            format!("Не удалось найти исполняемый файл Java в {}", target_dir.display())
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&java_bin, fs::Permissions::from_mode(0o755));
        }

        {
            let mut p = progress.lock().unwrap();
            p.stage = format!("Eclipse Temurin Java {} успешно установлена!", major_version);
            p.percent = 1.0;
            p.is_done = true;
        }

        Ok(java_bin)
    }
}

pub fn find_java_in_dir(root: &Path) -> Option<PathBuf> {
    let target_name = if cfg!(target_os = "windows") {
        "javaw.exe"
    } else {
        "java"
    };

    // 1. Direct check: root/bin/javaw.exe
    let direct = root.join("bin").join(target_name);
    if direct.exists() {
        return Some(direct);
    }

    // Windows fallback: root/bin/java.exe
    if cfg!(target_os = "windows") {
        let direct_java = root.join("bin").join("java.exe");
        if direct_java.exists() {
            return Some(direct_java);
        }
    }

    // 2. Recursive check inside subdirectories (up to 2 levels)
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let bin_check = path.join("bin").join(target_name);
                if bin_check.exists() {
                    return Some(bin_check);
                }
                if cfg!(target_os = "windows") {
                    let java_check = path.join("bin").join("java.exe");
                    if java_check.exists() {
                        return Some(java_check);
                    }
                }
            }
        }
    }

    None
}