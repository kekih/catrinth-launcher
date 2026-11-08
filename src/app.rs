use crate::catrinth_api::{CatrinthClient, ModHit};
use crate::config::LauncherConfig;
use crate::java_manager::{JavaManager, ALL_TEMURIN_VERSIONS};
use crate::launcher::{InstalledMod, MinecraftLauncher};
use crate::mod_loaders::{self, get_default_versions, ModLoaderType, ParsedVersion};
use crate::mojang_api::{DownloadProgress, MojangClient, VersionManifest};
use crate::theme::{
    COLOR_BG_CARD, COLOR_BG_HOVER, COLOR_BG_INPUT, COLOR_BG_MAIN, COLOR_BG_SIDEBAR,
    COLOR_BG_SURFACE, COLOR_BLUE, COLOR_BLUE_BG, COLOR_BORDER, COLOR_BORDER_LIGHT,
    COLOR_ORANGE, COLOR_ORANGE_BG, COLOR_PRIMARY, COLOR_PRIMARY_BG, COLOR_PRIMARY_HOVER,
    COLOR_PURPLE, COLOR_PURPLE_BG, COLOR_RED, COLOR_RED_BG, COLOR_TEXT_BODY,
    COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY, COLOR_TEXT_SUBTLE,
};
use eframe::egui;
use egui::{vec2, Align, Color32, Layout, Margin, RichText, Rounding, Stroke, Ui};
use std::path::PathBuf;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Play,
    ModsCatalog,
    InstalledMods,
    JavaRuntimes,
    Settings,
}

pub struct RunningGame {
    pub child: Child,
    pub version: String,
    pub pid: u32,
    pub log_path: PathBuf,
}

pub struct CatrinthApp {
    config: LauncherConfig,
    active_tab: Tab,

    // Play Tab State
    available_versions: Vec<String>,
    loader_filter: ModLoaderType,
    show_snapshots: bool,
    status_message: Option<(String, bool)>, // (msg, is_error)

    // Running Minecraft Process
    running_game: Option<RunningGame>,

    // Download & Launch State
    download_progress: Arc<Mutex<DownloadProgress>>,
    is_downloading: bool,
    manifest_cache: Arc<Mutex<Option<VersionManifest>>>,

    // Java Manager State
    java_download_progress: Arc<Mutex<DownloadProgress>>,
    is_installing_java: bool,

    // Mods Catalog State
    search_query: String,
    search_category: String,
    search_results: Vec<ModHit>,
    mod_status_message: Option<(String, bool)>,

    // Installed Mods State
    installed_mods: Vec<InstalledMod>,

    // Settings State
    status_connection: Option<String>,
}

impl CatrinthApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut config = LauncherConfig::load();
        let mut versions = MinecraftLauncher::get_installed_versions(&config);
        for def in get_default_versions() {
            if !versions.contains(&def) {
                versions.push(def);
            }
        }

        if config.selected_version.is_empty() {
            config.selected_version = "1.21.1 (Fabric)".to_string();
        }

        let mods = MinecraftLauncher::get_installed_mods(&config);

        let manifest_slot = Arc::new(Mutex::new(None));
        let manifest_slot_bg = Arc::clone(&manifest_slot);
        let versions_dir = config.versions_dir();

        // Async fetch Mojang version manifest on startup
        thread::spawn(move || {
            let mojang = MojangClient::new();
            if let Ok(manifest) = mojang.fetch_manifest(&versions_dir) {
                let mut slot = manifest_slot_bg.lock().unwrap();
                *slot = Some(manifest);
            }
        });

        let mut app = Self {
            config,
            active_tab: Tab::Play,
            available_versions: versions,
            loader_filter: ModLoaderType::All,
            show_snapshots: false,
            status_message: None,
            running_game: None,
            download_progress: Arc::new(Mutex::new(DownloadProgress::default())),
            is_downloading: false,
            manifest_cache: manifest_slot,
            java_download_progress: Arc::new(Mutex::new(DownloadProgress::default())),
            is_installing_java: false,
            search_query: String::new(),
            search_category: "mod".to_string(),
            search_results: Vec::new(),
            mod_status_message: None,
            installed_mods: mods,
            status_connection: None,
        };

        // Initial search for popular mods
        app.perform_search();
        app
    }

    fn apply_version_manifest(&mut self, manifest: &VersionManifest) {
        let mut list = self.available_versions.clone();

        for v in &manifest.versions {
            if v.version_type == "release"
                || (self.show_snapshots && v.version_type == "snapshot")
            {
                let v_name = format!("{} (Vanilla)", v.id);
                if !list.contains(&v_name) && !list.contains(&v.id) {
                    list.push(v_name);
                }
            }
        }

        self.available_versions = list;
    }

    fn perform_search(&mut self) {
        let client = CatrinthClient::new(self.config.api_proxy_url.clone());
        let q = self.search_query.trim().to_string();
        let cat = self.search_category.clone();

        match client.search_mods(&q, &cat, 20) {
            Ok(hits) => {
                self.search_results = hits;
                self.mod_status_message = None;
            }
            Err(e) => {
                self.mod_status_message = Some((e, true));
            }
        }
    }

    fn refresh_installed_mods(&mut self) {
        self.installed_mods = MinecraftLauncher::get_installed_mods(&self.config);
    }

    fn start_install_java(&mut self, major_version: u32, ctx: egui::Context) {
        self.is_installing_java = true;
        self.status_message = None;

        let progress_arc = Arc::clone(&self.java_download_progress);
        {
            let mut p = progress_arc.lock().unwrap();
            *p = DownloadProgress {
                stage: format!("Подготовка к установке Eclipse Temurin Java {}...", major_version),
                current: 0,
                total: 0,
                percent: 0.05,
                message: "Запрос ссылки к Adoptium API...".to_string(),
                is_done: false,
                error: None,
            };
        }

        let config_clone = self.config.clone();

        thread::spawn(move || {
            match JavaManager::download_and_install_temurin(major_version, progress_arc.clone()) {
                Ok(java_path) => {
                    let mut p = progress_arc.lock().unwrap();
                    p.stage = format!("Eclipse Temurin Java {} успешно установлена!", major_version);
                    p.percent = 1.0;
                    p.is_done = true;

                    let mut cfg = config_clone;
                    cfg.java_path = java_path.to_string_lossy().to_string();
                    let _ = cfg.save();
                }
                Err(e) => {
                    let mut p = progress_arc.lock().unwrap();
                    p.error = Some(e);
                    p.is_done = true;
                }
            }
            ctx.request_repaint();
        });
    }

    fn start_download_and_launch(&mut self, version: String, ctx: egui::Context) {
        self.is_downloading = true;
        self.status_message = None;

        let progress_arc = Arc::clone(&self.download_progress);
        {
            let mut p = progress_arc.lock().unwrap();
            *p = DownloadProgress {
                stage: format!("Подготовка к установке {}...", version),
                current: 0,
                total: 0,
                percent: 0.05,
                message: "Связь с серверами Mojang (piston-meta)...".to_string(),
                is_done: false,
                error: None,
            };
        }

        let config_clone = self.config.clone();

        thread::spawn(move || {
            let parsed = ParsedVersion::parse(&version);
            let game_dir = PathBuf::from(&config_clone.game_dir);
            let mojang = MojangClient::new();

            // 1. Manifest
            let manifest = match mojang.fetch_manifest(&config_clone.versions_dir()) {
                Ok(m) => m,
                Err(e) => {
                    let mut p = progress_arc.lock().unwrap();
                    p.error = Some(e);
                    p.is_done = true;
                    ctx.request_repaint();
                    return;
                }
            };

            // 2. Fetch base vanilla version details
            {
                let mut p = progress_arc.lock().unwrap();
                p.stage = format!("Загрузка спецификации Minecraft {}...", parsed.game_version);
                p.percent = 0.10;
            }
            ctx.request_repaint();

            let details = match mojang.fetch_version_details(&game_dir, &manifest, &parsed.game_version) {
                Ok(d) => d,
                Err(e) => {
                    let mut p = progress_arc.lock().unwrap();
                    p.error = Some(e);
                    p.is_done = true;
                    ctx.request_repaint();
                    return;
                }
            };

            // 3. Client JAR
            if let Err(e) = mojang.download_client_jar(&game_dir, &details, &progress_arc) {
                let mut p = progress_arc.lock().unwrap();
                p.error = Some(e);
                p.is_done = true;
                ctx.request_repaint();
                return;
            }

            // 4. Libraries
            if let Err(e) = mojang.download_libraries(&game_dir, &details, &progress_arc) {
                let mut p = progress_arc.lock().unwrap();
                p.error = Some(e);
                p.is_done = true;
                ctx.request_repaint();
                return;
            }

            // 5. Assets (textures, sounds)
            if let Err(e) = mojang.download_assets(&game_dir, &details, &progress_arc) {
                let mut p = progress_arc.lock().unwrap();
                p.error = Some(e);
                p.is_done = true;
                ctx.request_repaint();
                return;
            }

            // 6. Fabric loader installation if Fabric selected
            if parsed.loader == ModLoaderType::Fabric {
                if let Err(e) = mod_loaders::install_fabric(&parsed.game_version, &config_clone.game_dir, Arc::clone(&progress_arc)) {
                    let mut p = progress_arc.lock().unwrap();
                    p.error = Some(format!("Ошибка установки Fabric: {}", e));
                    p.is_done = true;
                    ctx.request_repaint();
                    return;
                }
            }

            // 7. Forge loader installation if Forge selected
            if parsed.loader == ModLoaderType::Forge {
                if let Some(bin) = JavaManager::find_best_java_for_minecraft(&config_clone.java_path, &parsed.game_version) {
                    if let Err(e) = mod_loaders::install_forge(&parsed.game_version, &config_clone.game_dir, &bin, Arc::clone(&progress_arc)) {
                        let mut p = progress_arc.lock().unwrap();
                        p.error = Some(format!("Ошибка установки Forge: {}", e));
                        p.is_done = true;
                        ctx.request_repaint();
                        return;
                    }
                } else {
                    let mut p = progress_arc.lock().unwrap();
                    p.error = Some("Для установки Forge необходима Java! Перейдите во вкладку Java Runtimes и установите Java 21 или Java 17.".to_string());
                    p.is_done = true;
                    ctx.request_repaint();
                    return;
                }
            }

            // 8. Launch!
            {
                let mut p = progress_arc.lock().unwrap();
                p.stage = format!("Все файлы {} проверены! Запуск...", version);
                p.percent = 1.0;
            }
            ctx.request_repaint();

            let mut p = progress_arc.lock().unwrap();
            p.message = "READY_TO_LAUNCH".to_string();
            p.is_done = true;
            ctx.request_repaint();
        });
    }
}

impl eframe::App for CatrinthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Check for fetched Mojang version manifest
        let manifest_opt = self.manifest_cache.try_lock().ok().and_then(|mut s| s.take());
        if let Some(manifest) = manifest_opt {
            self.apply_version_manifest(&manifest);
        }

        // 2. Monitor running game process
        if let Some(ref mut game) = self.running_game {
            ctx.request_repaint();
            match game.child.try_wait() {
                Ok(Some(status)) => {
                    let ver = game.version.clone();
                    let log_path = game.log_path.clone();
                    self.running_game = None;

                    if status.success() {
                        self.status_message = Some((
                            format!("Minecraft {} завершил работу (код 0).", ver),
                            false,
                        ));
                    } else {
                        let tail = MinecraftLauncher::read_log_tail(&log_path, 8);
                        let err_text = if !tail.is_empty() {
                            format!(
                                "Minecraft {} закрылся с ошибкой (код {:?}):\n\n{}",
                                ver,
                                status.code(),
                                tail
                            )
                        } else {
                            format!(
                                "Minecraft {} закрылся с ошибкой (код {:?}). Проверьте лог игры.",
                                ver,
                                status.code()
                            )
                        };
                        self.status_message = Some((err_text, true));
                    }
                }
                Ok(None) => {
                    // Still running
                }
                Err(e) => {
                    self.status_message = Some((
                        format!("Ошибка отслеживания процесса игры: {}", e),
                        true,
                    ));
                    self.running_game = None;
                }
            }
        }

        // 3. Monitor game downloader status
        if self.is_downloading {
            ctx.request_repaint();
            let (done, err, msg) = {
                let p = self.download_progress.lock().unwrap();
                (p.is_done, p.error.clone(), p.message.clone())
            };

            if done {
                self.is_downloading = false;
                if let Some(e) = err {
                    self.status_message = Some((e, true));
                } else if msg == "READY_TO_LAUNCH" {
                    let ver = self.config.selected_version.clone();
                    match MinecraftLauncher::launch(&self.config, &ver) {
                        Ok((child, log_path, ok_msg)) => {
                            let pid = child.id();
                            self.running_game = Some(RunningGame {
                                child,
                                version: ver,
                                pid,
                                log_path,
                            });
                            self.status_message = Some((ok_msg, false));
                        }
                        Err(e) => {
                            self.status_message = Some((e, true));
                        }
                    }
                }
                self.available_versions =
                    MinecraftLauncher::get_installed_versions(&self.config);
            }
        }

        // 4. Monitor Java installation status
        if self.is_installing_java {
            ctx.request_repaint();
            let (done, err) = {
                let p = self.java_download_progress.lock().unwrap();
                (p.is_done, p.error.clone())
            };

            if done {
                self.is_installing_java = false;
                if let Some(e) = err {
                    self.status_message = Some((e, true));
                } else {
                    if let Some(bin) = JavaManager::get_temurin_java_bin(21) {
                        self.config.java_path = bin.to_string_lossy().to_string();
                        let _ = self.config.save();
                        self.status_message = Some((
                            format!("Eclipse Temurin Java 21 успешно установлена и выбрана:\n{}", bin.display()),
                            false,
                        ));
                    }
                }
            }
        }

        // ====================================================================
        // MODRINTH APP NAVIGATION SIDEBAR
        // ====================================================================
        egui::SidePanel::left("modrinth_sidebar")
            .exact_width(240.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_SIDEBAR)
                    .inner_margin(Margin {
                        left: 14.0,
                        right: 14.0,
                        top: 20.0,
                        bottom: 16.0,
                    })
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER)),
            )
            .show(ctx, |ui| {
                // Top Brand Header (Clean vector drawn emblem, no broken fonts)
                ui.horizontal(|ui| {
                    let (emblem_rect, _) = ui.allocate_exact_size(vec2(36.0, 36.0), egui::Sense::hover());
                    ui.painter().rect_filled(emblem_rect, Rounding::same(8.0), COLOR_PRIMARY);
                    ui.painter().text(
                        emblem_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "C",
                        egui::FontId::proportional(22.0),
                        Color32::from_rgb(8, 20, 12),
                    );

                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Catrinth").color(COLOR_TEXT_PRIMARY).size(18.0).strong());
                            egui::Frame::none()
                                .fill(COLOR_PRIMARY)
                                .rounding(Rounding::same(4.0))
                                .inner_margin(Margin::symmetric(5.0, 1.5))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("APP")
                                            .color(Color32::from_rgb(8, 20, 12))
                                            .size(10.0)
                                            .strong(),
                                    );
                                });
                        });
                        ui.label(RichText::new("MINECRAFT HUB").color(COLOR_TEXT_SUBTLE).size(9.5).strong());
                    });
                });

                ui.add_space(20.0);

                // Thin divider line
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, COLOR_BORDER);

                ui.add_space(16.0);

                // Sidebar Navigation Items
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());

                    sidebar_pill_button(ui, "▶", "Главная / Играть", None, self.active_tab == Tab::Play, || {
                        self.active_tab = Tab::Play;
                    });

                    ui.add_space(4.0);
                    sidebar_pill_button(ui, "⊞", "Каталог модов", None, self.active_tab == Tab::ModsCatalog, || {
                        self.active_tab = Tab::ModsCatalog;
                    });

                    ui.add_space(4.0);
                    let count_badge = format!("{}", self.installed_mods.len());
                    sidebar_pill_button(ui, "▦", "Моды игры", Some(&count_badge), self.active_tab == Tab::InstalledMods, || {
                        self.active_tab = Tab::InstalledMods;
                        self.refresh_installed_mods();
                    });

                    ui.add_space(4.0);
                    sidebar_pill_button(ui, "☕", "Среда Java", None, self.active_tab == Tab::JavaRuntimes, || {
                        self.active_tab = Tab::JavaRuntimes;
                    });

                    ui.add_space(4.0);
                    sidebar_pill_button(ui, "⚙", "Настройки", None, self.active_tab == Tab::Settings, || {
                        self.active_tab = Tab::Settings;
                    });
                });

                // Bottom User Profile Card
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    ui.add_space(8.0);

                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let initial = self.config.username.chars().next().unwrap_or('P').to_uppercase().to_string();
                                let (av_rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::hover());
                                ui.painter().rect_filled(av_rect, Rounding::same(6.0), COLOR_BG_SURFACE);
                                ui.painter().rect_stroke(av_rect, Rounding::same(6.0), Stroke::new(1.0, COLOR_PRIMARY));
                                ui.painter().text(
                                    av_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    &initial,
                                    egui::FontId::proportional(15.0),
                                    COLOR_PRIMARY,
                                );

                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(&self.config.username)
                                            .color(COLOR_TEXT_PRIMARY)
                                            .size(13.5)
                                            .strong(),
                                    );
                                    ui.horizontal(|ui| {
                                        let (dot_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                                        ui.painter().circle_filled(dot_rect.center(), 3.5, COLOR_PRIMARY);
                                        ui.label(RichText::new("Catrinth Profile").color(COLOR_TEXT_MUTED).size(11.0));
                                    });
                                });
                            });

                            ui.add_space(8.0);
                            let ram_gb = self.config.ram_max_mb as f32 / 1024.0;
                            egui::Frame::none()
                                .fill(COLOR_PRIMARY_BG)
                                .rounding(Rounding::same(5.0))
                                .inner_margin(Margin::symmetric(8.0, 3.5))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("⚡ ОЗУ: {:.1} GB выделено", ram_gb))
                                            .color(COLOR_PRIMARY)
                                            .size(11.0)
                                            .strong(),
                                    );
                                });
                        });
                });
            });

        // Central Main Panel with Scroll Area to prevent window cut-off
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_MAIN)
                    .inner_margin(Margin::symmetric(24.0, 20.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_tab {
                        Tab::Play => self.render_play_tab(ui, ctx),
                        Tab::ModsCatalog => self.render_mods_tab(ui),
                        Tab::InstalledMods => self.render_installed_mods_tab(ui),
                        Tab::JavaRuntimes => self.render_java_tab(ui, ctx),
                        Tab::Settings => self.render_settings_tab(ui),
                    });
            });
    }
}

// Custom modern sidebar pill button
fn sidebar_pill_button(
    ui: &mut Ui,
    icon: &str,
    label: &str,
    badge: Option<&str>,
    active: bool,
    mut on_click: impl FnMut(),
) {
    let height = 40.0;
    let full_width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(full_width, height), egui::Sense::click());

    let bg_color = if active {
        COLOR_BG_SURFACE
    } else if response.hovered() {
        COLOR_BG_HOVER
    } else {
        Color32::TRANSPARENT
    };

    ui.painter().rect_filled(rect, Rounding::same(8.0), bg_color);

    if active {
        let bar_rect = egui::Rect::from_min_size(
            rect.min + vec2(2.0, 8.0),
            vec2(3.5, height - 16.0),
        );
        ui.painter().rect_filled(bar_rect, Rounding::same(2.0), COLOR_PRIMARY);
    }

    let text_color = if active {
        COLOR_PRIMARY
    } else if response.hovered() {
        COLOR_TEXT_PRIMARY
    } else {
        COLOR_TEXT_BODY
    };

    let content_rect = rect.shrink2(vec2(14.0, 0.0));
    ui.allocate_ui_at_rect(content_rect, |ui| {
        ui.horizontal_centered(|ui| {
            ui.label(RichText::new(icon).color(text_color).size(15.0).strong());
            ui.add_space(8.0);
            ui.label(
                RichText::new(label)
                    .color(text_color)
                    .size(13.5)
                    .strong(),
            );

            if let Some(b) = badge {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::Frame::none()
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::symmetric(6.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(b).color(COLOR_TEXT_BODY).size(11.0).strong());
                        });
                });
            }
        });
    });

    if response.clicked() {
        on_click();
    }
}

// Clean badge with custom painted solid dot (no emoji missing square boxes)
fn render_badge(ui: &mut Ui, text: &str, dot_color: Option<Color32>, bg: Color32, text_color: Color32) {
    egui::Frame::none()
        .fill(bg)
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(col) = dot_color {
                    let (dot_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 3.5, col);
                    ui.add_space(2.0);
                }
                ui.label(RichText::new(text).color(text_color).size(11.5).strong());
            });
        });
}

impl CatrinthApp {
    // ------------------------------------------------------------------------
    // TAB 1: PLAY & PROCESS CONTROL (MODRINTH APP HERO DASHBOARD)
    // ------------------------------------------------------------------------
    fn render_play_tab(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let is_running = self.running_game.is_some();
        let is_installed = MinecraftLauncher::is_version_installed(
            &self.config,
            &self.config.selected_version,
        );
        let parsed = ParsedVersion::parse(&self.config.selected_version);

        // ====================================================================
        // HERO BANNER CARD (Instance Header)
        // ====================================================================
        egui::Frame::none()
            .fill(COLOR_BG_CARD)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER_LIGHT))
            .rounding(Rounding::same(14.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Modern stylized "MC" instance tile (always sharp)
                    let (icon_rect, _) = ui.allocate_exact_size(vec2(54.0, 54.0), egui::Sense::hover());
                    ui.painter().rect_filled(icon_rect, Rounding::same(10.0), COLOR_BG_SURFACE);
                    ui.painter().rect_stroke(icon_rect, Rounding::same(10.0), Stroke::new(1.0, COLOR_BORDER_LIGHT));
                    ui.painter().text(
                        icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "MC",
                        egui::FontId::proportional(22.0),
                        COLOR_PRIMARY,
                    );

                    ui.add_space(14.0);

                    // Title & Badges
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Minecraft {}", parsed.game_version))
                                    .color(COLOR_TEXT_PRIMARY)
                                    .size(24.0)
                                    .strong(),
                            );

                            ui.add_space(4.0);

                            // Mod Loader Pill Badge
                            match parsed.loader {
                                ModLoaderType::Fabric => {
                                    render_badge(ui, "FABRIC", Some(COLOR_PURPLE), COLOR_PURPLE_BG, COLOR_PURPLE);
                                }
                                ModLoaderType::Forge => {
                                    render_badge(ui, "FORGE", Some(COLOR_ORANGE), COLOR_ORANGE_BG, COLOR_ORANGE);
                                }
                                ModLoaderType::Vanilla | ModLoaderType::All => {
                                    render_badge(ui, "VANILLA", Some(COLOR_PRIMARY), COLOR_PRIMARY_BG, COLOR_PRIMARY);
                                }
                            }

                            // Status Pill Badge
                            if is_running {
                                render_badge(ui, "В ИГРЕ", Some(COLOR_PRIMARY), COLOR_PRIMARY_BG, COLOR_PRIMARY);
                            } else if is_installed {
                                render_badge(ui, "Готова к запуску", Some(COLOR_PRIMARY), COLOR_BG_SURFACE, COLOR_TEXT_BODY);
                            } else {
                                render_badge(ui, "Требуется загрузка", Some(COLOR_PURPLE), COLOR_PURPLE_BG, COLOR_TEXT_BODY);
                            }
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Сборка Minecraft").color(COLOR_TEXT_BODY).size(12.5));
                            ui.label(RichText::new("•").color(COLOR_BORDER_LIGHT).size(12.0));
                            ui.label(RichText::new(format!("Папка: {}", self.config.game_dir)).color(COLOR_TEXT_MUTED).size(12.0));
                        });
                    });
                });

                ui.add_space(20.0);

                // Thin separator
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, COLOR_BORDER);

                ui.add_space(18.0);

                // Prominent Action Button Row
                let ver = self.config.selected_version.clone();
                let ctx_clone = ctx.clone();

                if self.is_downloading {
                    let p = self.download_progress.lock().unwrap().clone();
                    egui::Frame::none()
                        .fill(COLOR_BG_SURFACE)
                        .stroke(Stroke::new(1.0_f32, COLOR_PRIMARY))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(16.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&p.stage).color(COLOR_PRIMARY).strong().size(14.5));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(RichText::new(format!("{:.0}%", p.percent * 100.0)).color(COLOR_PRIMARY).strong().size(14.5));
                                });
                            });
                            ui.add_space(8.0);
                            ui.add(egui::ProgressBar::new(p.percent).animate(true));
                            if !p.message.is_empty() && p.message != "READY_TO_LAUNCH" {
                                ui.add_space(6.0);
                                ui.label(RichText::new(&p.message).color(COLOR_TEXT_BODY).size(12.5));
                            }
                        });
                } else if is_running {
                    let pid = self.running_game.as_ref().map(|g| g.pid).unwrap_or(0);
                    let log_path = self.running_game.as_ref().map(|g| g.log_path.clone());

                    ui.horizontal(|ui| {
                        let kill_btn = egui::Button::new(
                            RichText::new(format!("■  ОСТАНОВИТЬ MINECRAFT (PID: {})", pid))
                                .color(Color32::WHITE)
                                .size(15.5)
                                .strong(),
                        )
                        .fill(COLOR_RED)
                        .rounding(Rounding::same(10.0))
                        .min_size(vec2(320.0, 48.0));

                        if ui.add(kill_btn).clicked() {
                            if let Some(mut game) = self.running_game.take() {
                                let _ = game.child.kill();
                                self.status_message = Some((
                                    format!("Процесс Minecraft {} (PID: {}) остановлен пользователем.", ver, pid),
                                    false,
                                ));
                            }
                        }

                        if let Some(path) = log_path {
                            let log_btn = egui::Button::new(
                                RichText::new("Лог игры")
                                    .color(COLOR_TEXT_PRIMARY)
                                    .size(13.5)
                                    .strong(),
                            )
                            .fill(COLOR_BG_SURFACE)
                            .rounding(Rounding::same(10.0))
                            .min_size(vec2(130.0, 48.0));

                            if ui.add(log_btn).clicked() {
                                let _ = open::that(&path);
                            }
                        }
                    });
                } else if is_installed {
                    ui.horizontal(|ui| {
                        let play_btn = egui::Button::new(
                            RichText::new("▶  ИГРАТЬ В MINECRAFT")
                                .color(Color32::from_rgb(8, 20, 12))
                                .size(16.0)
                                .strong(),
                        )
                        .fill(COLOR_PRIMARY)
                        .rounding(Rounding::same(10.0))
                        .min_size(vec2(260.0, 48.0));

                        if ui.add(play_btn).clicked() {
                            let _ = self.config.save();
                            match MinecraftLauncher::launch(&self.config, &ver) {
                                Ok((child, log_path, msg)) => {
                                    let pid = child.id();
                                    self.running_game = Some(RunningGame {
                                        child,
                                        version: ver.clone(),
                                        pid,
                                        log_path,
                                    });
                                    self.status_message = Some((msg, false));
                                }
                                Err(err) => self.status_message = Some((err, true)),
                            }
                        }

                        let folder_btn = egui::Button::new(
                            RichText::new("Папка игры")
                                .color(COLOR_TEXT_PRIMARY)
                                .size(13.5)
                                .strong(),
                        )
                        .fill(COLOR_BG_SURFACE)
                        .rounding(Rounding::same(10.0))
                        .min_size(vec2(130.0, 48.0));

                        if ui.add(folder_btn).clicked() {
                            let _ = open::that(&self.config.game_dir);
                        }

                        let update_btn = egui::Button::new(
                            RichText::new("Проверить файлы")
                                .color(COLOR_TEXT_BODY)
                                .size(13.5),
                        )
                        .fill(COLOR_BG_SURFACE)
                        .rounding(Rounding::same(10.0))
                        .min_size(vec2(140.0, 48.0));

                        if ui.add(update_btn).clicked() {
                            let _ = self.config.save();
                            self.start_download_and_launch(ver.clone(), ctx_clone);
                        }

                        let log_file = PathBuf::from(&self.config.game_dir).join("logs").join("catrinth-game.log");
                        if log_file.exists() {
                            let log_btn = egui::Button::new(
                                RichText::new("Лог")
                                    .color(COLOR_TEXT_BODY)
                                    .size(13.5),
                            )
                            .fill(COLOR_BG_SURFACE)
                            .rounding(Rounding::same(10.0))
                            .min_size(vec2(80.0, 48.0));

                            if ui.add(log_btn).clicked() {
                                let _ = open::that(&log_file);
                            }
                        }
                    });
                } else {
                    let dl_btn = egui::Button::new(
                        RichText::new(format!("СКАЧАТЬ И ИГРАТЬ ({})", ver))
                            .color(Color32::from_rgb(8, 20, 12))
                            .size(16.0)
                            .strong(),
                    )
                    .fill(COLOR_PRIMARY)
                    .rounding(Rounding::same(10.0))
                    .min_size(vec2(340.0, 48.0));

                    if ui.add(dl_btn).clicked() {
                        let _ = self.config.save();
                        self.start_download_and_launch(ver.clone(), ctx_clone);
                    }
                }

                // Alert / Status Message Display
                if let Some((msg, is_err)) = &self.status_message {
                    ui.add_space(14.0);
                    let (bg, border, text_col) = if *is_err {
                        (COLOR_RED_BG, COLOR_RED, COLOR_RED)
                    } else {
                        (COLOR_PRIMARY_BG, COLOR_PRIMARY, COLOR_PRIMARY)
                    };

                    egui::Frame::none()
                        .fill(bg)
                        .stroke(Stroke::new(1.0_f32, border))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(msg).color(text_col).size(13.0).strong());
                        });
                }
            });

        ui.add_space(20.0);

        // ====================================================================
        // MODERN 2-COLUMN DASHBOARD (Guaranteed Vertical Flow in Each Column)
        // ====================================================================
        ui.columns(2, |columns| {
            // ----------------------------------------------------------------
            // COLUMN 1: КАРТОЧКА «КОНФИГУРАЦИЯ ЗАПУСКА»
            // ----------------------------------------------------------------
            egui::Frame::none()
                .fill(COLOR_BG_CARD)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(Rounding::same(12.0))
                .inner_margin(Margin::same(20.0))
                .show(&mut columns[0], |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Конфигурация запуска")
                                .color(COLOR_TEXT_PRIMARY)
                                .size(16.5)
                                .strong(),
                        );

                        ui.add_space(14.0);

                        // Username Input
                        ui.label(RichText::new("Никнейм игрока:").color(COLOR_TEXT_BODY).size(13.0).strong());
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.username)
                                .desired_width(ui.available_width())
                                .hint_text("Ваш ник в игре"),
                        );

                        ui.add_space(14.0);

                        // Loader Selector Chips
                        ui.label(RichText::new("Загрузчик модов:").color(COLOR_TEXT_BODY).size(13.0).strong());
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            for (filter, label, dot_color) in [
                                (ModLoaderType::All, "Все", None),
                                (ModLoaderType::Fabric, "Fabric", Some(COLOR_PURPLE)),
                                (ModLoaderType::Forge, "Forge", Some(COLOR_ORANGE)),
                                (ModLoaderType::Vanilla, "Vanilla", Some(COLOR_PRIMARY)),
                            ] {
                                let is_active = self.loader_filter == filter;
                                let (bg, stroke, text_color) = if is_active {
                                    (COLOR_PRIMARY, Stroke::new(1.0_f32, COLOR_PRIMARY), Color32::from_rgb(8, 20, 12))
                                } else {
                                    (COLOR_BG_SURFACE, Stroke::new(1.0_f32, COLOR_BORDER), COLOR_TEXT_BODY)
                                };

                                let btn = egui::Button::new(
                                    RichText::new(label)
                                        .color(text_color)
                                        .size(12.5)
                                        .strong(),
                                )
                                .fill(bg)
                                .stroke(stroke)
                                .rounding(Rounding::same(6.0));

                                if ui.add(btn).clicked() {
                                    self.loader_filter = filter;
                                }
                            }
                        });

                        ui.add_space(14.0);

                        // Version Selection Dropdown
                        ui.label(RichText::new("Версия Minecraft:").color(COLOR_TEXT_BODY).size(13.0).strong());
                        ui.add_space(4.0);
                        egui::ComboBox::from_id_source("version_combobox_main")
                            .selected_text(&self.config.selected_version)
                            .width(ui.available_width() - 8.0)
                            .show_ui(ui, |ui| {
                                let filtered: Vec<String> = self
                                    .available_versions
                                    .iter()
                                    .filter(|v| {
                                        let p = ParsedVersion::parse(v);
                                        match self.loader_filter {
                                            ModLoaderType::All => true,
                                            ModLoaderType::Fabric => p.loader == ModLoaderType::Fabric,
                                            ModLoaderType::Forge => p.loader == ModLoaderType::Forge,
                                            ModLoaderType::Vanilla => p.loader == ModLoaderType::Vanilla,
                                        }
                                    })
                                    .cloned()
                                    .collect();

                                for v in &filtered {
                                    let tag = if MinecraftLauncher::is_version_installed(&self.config, v) {
                                        "  [Готова]"
                                    } else {
                                        "  [Скачать]"
                                    };
                                    ui.selectable_value(
                                        &mut self.config.selected_version,
                                        v.clone(),
                                        format!("{}{}", v, tag),
                                    );
                                }
                            });

                        ui.add_space(10.0);
                        ui.checkbox(&mut self.show_snapshots, "Показывать экспериментальные снапшоты");
                    });
                });

            // ----------------------------------------------------------------
            // COLUMN 2: КАРТОЧКА «РЕСУРСЫ И СРЕДА»
            // ----------------------------------------------------------------
            egui::Frame::none()
                .fill(COLOR_BG_CARD)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(Rounding::same(12.0))
                .inner_margin(Margin::same(20.0))
                .show(&mut columns[1], |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Ресурсы и среда")
                                .color(COLOR_TEXT_PRIMARY)
                                .size(16.5)
                                .strong(),
                        );

                        ui.add_space(14.0);

                        // RAM Allocation Header
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Оперативная память (ОЗУ):").color(COLOR_TEXT_BODY).size(13.0).strong());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let gb = self.config.ram_max_mb as f32 / 1024.0;
                                ui.label(
                                    RichText::new(format!("{:.1} GB ({} MB)", gb, self.config.ram_max_mb))
                                        .color(COLOR_PRIMARY)
                                        .size(13.5)
                                        .strong(),
                                );
                            });
                        });

                        ui.add_space(6.0);
                        // RAM Preset Buttons
                        ui.horizontal(|ui| {
                            for gb in [2, 4, 6, 8, 12, 16] {
                                let mb = gb * 1024;
                                let is_active = self.config.ram_max_mb == mb;
                                let (bg, text_color) = if is_active {
                                    (COLOR_PRIMARY, Color32::from_rgb(8, 20, 12))
                                } else {
                                    (COLOR_BG_SURFACE, COLOR_TEXT_BODY)
                                };

                                let btn = egui::Button::new(
                                    RichText::new(format!("{}G", gb))
                                        .color(text_color)
                                        .size(12.0)
                                        .strong(),
                                )
                                .fill(bg)
                                .rounding(Rounding::same(6.0));

                                if ui.add(btn).clicked() {
                                    self.config.ram_max_mb = mb;
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.add(
                            egui::Slider::new(&mut self.config.ram_max_mb, 1024..=32768)
                                .step_by(512.0)
                                .show_value(false),
                        );

                        ui.add_space(16.0);

                        // Java Environment Card
                        ui.label(RichText::new("Среда выполнения Java:").color(COLOR_TEXT_BODY).size(13.0).strong());
                        ui.add_space(4.0);

                        let java_disp = if self.config.java_path.is_empty() {
                            "Автоматический выбор (Temurin / PATH)".to_string()
                        } else {
                            let p = PathBuf::from(&self.config.java_path);
                            p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| self.config.java_path.clone())
                        };

                        egui::Frame::none()
                            .fill(COLOR_BG_SURFACE)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::same(10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&java_disp).color(COLOR_TEXT_PRIMARY).size(12.5).strong());
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.add(
                                            egui::Button::new(
                                                RichText::new("Настроить Java →")
                                                    .color(COLOR_PRIMARY)
                                                    .size(12.0)
                                                    .strong(),
                                            )
                                            .fill(COLOR_PRIMARY_BG)
                                            .stroke(Stroke::new(1.0_f32, COLOR_PRIMARY))
                                            .rounding(Rounding::same(6.0)),
                                        ).clicked() {
                                            self.active_tab = Tab::JavaRuntimes;
                                        }
                                    });
                                });
                            });

                        ui.add_space(16.0);

                        // Game Directory
                        ui.label(RichText::new("Папка игры:").color(COLOR_TEXT_BODY).size(13.0).strong());
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&self.config.game_dir).color(COLOR_TEXT_MUTED).size(11.5));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(
                                    egui::Button::new("Открыть")
                                        .fill(COLOR_BG_SURFACE)
                                        .rounding(Rounding::same(6.0)),
                                ).clicked() {
                                    let _ = open::that(&self.config.game_dir);
                                }
                            });
                        });
                    });
                });
        });
    }

    // ------------------------------------------------------------------------
    // TAB 2: JAVA RUNTIMES & ECLIPSE TEMURIN OPENJDK DOWNLOADER
    // ------------------------------------------------------------------------
    fn render_java_tab(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.label(RichText::new("Среды выполнения Java (Adoptium Eclipse Temurin)").size(22.0).strong().color(COLOR_TEXT_PRIMARY));
        ui.add_space(4.0);
        ui.label(
            RichText::new("Официальный проверенный OpenJDK от Eclipse Foundation с оптимизацией для Minecraft. Скачивается прямо в папку runtime/ лаунчера.")
                .color(COLOR_TEXT_MUTED)
                .size(13.0),
        );

        ui.add_space(16.0);

        // Progress bar for Java Installation
        if self.is_installing_java {
            egui::Frame::none()
                .fill(COLOR_BG_CARD)
                .stroke(Stroke::new(1.0_f32, COLOR_PRIMARY))
                .rounding(Rounding::same(10.0))
                .inner_margin(Margin::same(16.0))
                .show(ui, |ui| {
                    let p = self.java_download_progress.lock().unwrap().clone();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&p.stage).color(COLOR_PRIMARY).strong().size(14.5));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(format!("{:.0}%", p.percent * 100.0)).color(COLOR_PRIMARY).strong().size(14.5));
                        });
                    });
                    ui.add_space(8.0);
                    ui.add(egui::ProgressBar::new(p.percent).animate(true));
                });
            ui.add_space(16.0);
        }

        // Active Java Status Card
        egui::Frame::none()
            .fill(COLOR_BG_CARD)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Активный исполняемый файл Java:").strong().color(COLOR_TEXT_PRIMARY));
                    let current = if self.config.java_path.is_empty() {
                        "Автоматический выбор (runtime/ или системный PATH)".to_string()
                    } else {
                        self.config.java_path.clone()
                    };
                    ui.label(RichText::new(current).color(COLOR_PRIMARY).strong());
                });
            });

        ui.add_space(16.0);

        for info in ALL_TEMURIN_VERSIONS {
            let has_temurin = JavaManager::is_temurin_installed(info.version);
            let is_active = if let Some(bin) = JavaManager::get_temurin_java_bin(info.version) {
                self.config.java_path == bin.to_string_lossy().to_string()
            } else {
                false
            };

            egui::Frame::none()
                .fill(COLOR_BG_CARD)
                .stroke(Stroke::new(1.0_f32, if is_active { COLOR_PRIMARY } else { COLOR_BORDER }))
                .rounding(Rounding::same(10.0))
                .inner_margin(Margin::same(18.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(info.title).size(16.0).strong().color(COLOR_TEXT_PRIMARY));
                                render_badge(ui, info.tag, None, COLOR_PRIMARY_BG, COLOR_PRIMARY);

                                if is_active {
                                    render_badge(ui, "АКТИВНАЯ", Some(COLOR_PRIMARY), COLOR_PRIMARY, Color32::from_rgb(8, 20, 12));
                                } else if has_temurin {
                                    render_badge(ui, "Установлена", Some(COLOR_PRIMARY), COLOR_BG_SURFACE, COLOR_PRIMARY);
                                }
                            });
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(info.description)
                                    .color(COLOR_TEXT_MUTED)
                                    .size(12.5),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if has_temurin {
                                if !is_active {
                                    let btn = egui::Button::new(
                                        RichText::new("Использовать эту Java")
                                            .color(COLOR_TEXT_PRIMARY)
                                            .size(13.0)
                                            .strong(),
                                    )
                                    .fill(COLOR_BG_SURFACE)
                                    .stroke(Stroke::new(1.0_f32, COLOR_PRIMARY))
                                    .rounding(Rounding::same(8.0))
                                    .min_size(vec2(170.0, 36.0));

                                    if ui.add(btn).clicked() {
                                        if let Some(bin) = JavaManager::get_temurin_java_bin(info.version) {
                                            self.config.java_path = bin.to_string_lossy().to_string();
                                            let _ = self.config.save();
                                        }
                                    }
                                } else {
                                    ui.label(RichText::new("✓ Выбрана").color(COLOR_PRIMARY).strong().size(14.0));
                                }
                            } else {
                                let is_highlight = info.version == 21 || info.version == 25;
                                let btn = egui::Button::new(
                                    RichText::new(format!("Установить Java {}", info.version))
                                        .color(if is_highlight { Color32::from_rgb(8, 20, 12) } else { COLOR_TEXT_PRIMARY })
                                        .size(13.0)
                                        .strong(),
                                )
                                .fill(if is_highlight { COLOR_PRIMARY } else { COLOR_BG_SURFACE })
                                .rounding(Rounding::same(8.0))
                                .min_size(vec2(180.0, 36.0));

                                if ui.add(btn).clicked() && !self.is_installing_java {
                                    self.start_install_java(info.version, ctx.clone());
                                }
                            }
                        });
                    });
                });

            ui.add_space(10.0);
        }
    }

    // ------------------------------------------------------------------------
    // TAB 3: CATRINTH MODS CATALOG (DISCOVER & INSTALL)
    // ------------------------------------------------------------------------
    fn render_mods_tab(&mut self, ui: &mut Ui) {
        // Search Header
        egui::Frame::none()
            .fill(COLOR_BG_CARD)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(16.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let edit = egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Поиск модов, шейдеров, ресурс-паков в Catrinth...")
                        .desired_width(380.0);

                    if ui.add(edit).lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.perform_search();
                    }

                    // Category Pill Selector
                    for (cat_id, cat_label) in [
                        ("mod", "Моды"),
                        ("shader", "Шейдеры"),
                        ("resourcepack", "Текстуры"),
                        ("datapack", "Датапаки"),
                    ] {
                        let is_active = self.search_category == cat_id;
                        let (bg, stroke, text_color) = if is_active {
                            (COLOR_PRIMARY, Stroke::new(1.0_f32, COLOR_PRIMARY), Color32::from_rgb(8, 20, 12))
                        } else {
                            (COLOR_BG_SURFACE, Stroke::new(1.0_f32, COLOR_BORDER), COLOR_TEXT_BODY)
                        };

                        let btn = egui::Button::new(
                            RichText::new(cat_label)
                                .color(text_color)
                                .size(12.5)
                                .strong(),
                        )
                        .fill(bg)
                        .stroke(stroke)
                        .rounding(Rounding::same(6.0));

                        if ui.add(btn).clicked() {
                            self.search_category = cat_id.to_string();
                            self.perform_search();
                        }
                    }

                    if ui.add(
                        egui::Button::new(
                            RichText::new("Найти")
                                .color(Color32::from_rgb(8, 20, 12))
                                .size(13.0)
                                .strong(),
                        )
                        .fill(COLOR_PRIMARY)
                        .rounding(Rounding::same(6.0))
                        .min_size(vec2(80.0, 32.0)),
                    ).clicked() {
                        self.perform_search();
                    }
                });
            });

        ui.add_space(14.0);

        if let Some((msg, is_err)) = &self.mod_status_message {
            let col = if *is_err { COLOR_RED } else { COLOR_PRIMARY };
            ui.label(RichText::new(msg).color(col).strong().size(13.0));
            ui.add_space(8.0);
        }

        let mut download_slug_action: Option<String> = None;
        let hits = self.search_results.clone();

        if hits.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new("Ничего не найдено").color(COLOR_TEXT_MUTED).size(16.0));
            });
        } else {
            for hit in &hits {
                let title = &hit.title;
                let p_type = hit.project_type.as_deref().unwrap_or("mod");
                let author = hit.author.as_deref().unwrap_or("Автор");
                let desc = hit.description.as_deref().unwrap_or("");
                let dl_count = hit.downloads.unwrap_or(0);
                let slug = hit.slug.as_deref().unwrap_or(&hit.title);

                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(16.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(title).color(COLOR_TEXT_PRIMARY).size(16.0).strong());
                                    render_badge(ui, p_type, None, COLOR_PURPLE_BG, COLOR_PURPLE);
                                    ui.label(RichText::new(format!("от {}", author)).color(COLOR_TEXT_MUTED).size(12.5));
                                });

                                if !desc.is_empty() {
                                    ui.add_space(4.0);
                                    ui.label(RichText::new(desc).color(COLOR_TEXT_BODY).size(13.0));
                                }

                                ui.add_space(6.0);
                                ui.label(RichText::new(format!("Скачиваний: {}", dl_count)).color(COLOR_TEXT_MUTED).size(11.5));
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let download_btn = egui::Button::new(
                                    RichText::new("Скачать")
                                        .color(Color32::from_rgb(8, 20, 12))
                                        .size(13.5)
                                        .strong(),
                                )
                                .fill(COLOR_PRIMARY)
                                .rounding(Rounding::same(8.0))
                                .min_size(vec2(110.0, 36.0));

                                if ui.add(download_btn).clicked() {
                                    download_slug_action = Some(slug.to_string());
                                }
                            });
                        });
                    });
                ui.add_space(8.0);
            }
        }

        if let Some(slug) = download_slug_action {
            let client = CatrinthClient::new(self.config.api_proxy_url.clone());
            match client.get_project_versions(&slug) {
                Ok(versions) => {
                    if let Some(first_ver) = versions.first() {
                        if let Some(file) = first_ver.files.iter().find(|f| f.primary.unwrap_or(false)).or_else(|| first_ver.files.first()) {
                            let mods_dir = self.config.mods_dir();
                            let target = mods_dir.join(&file.filename);
                            match client.download_file(&file.url, &target) {
                                Ok(_) => {
                                    self.mod_status_message = Some((
                                        format!("Мод {} успешно сохранён в папку mods!", file.filename),
                                        false,
                                    ));
                                    self.refresh_installed_mods();
                                }
                                Err(e) => {
                                    self.mod_status_message = Some((e, true));
                                }
                            }
                        } else {
                            self.mod_status_message = Some(("У этой версии нет файлов".to_string(), true));
                        }
                    } else {
                        self.mod_status_message = Some(("Версии не найдены".to_string(), true));
                    }
                }
                Err(e) => {
                    self.mod_status_message = Some((e, true));
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // TAB 4: INSTALLED MODS IN .MINECRAFT/MODS
    // ------------------------------------------------------------------------
    fn render_installed_mods_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Установленные моды ({})", self.installed_mods.len()))
                    .size(20.0)
                    .strong()
                    .color(COLOR_TEXT_PRIMARY),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(
                    egui::Button::new("Папка mods")
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(8.0)),
                ).clicked() {
                    let mods_dir = self.config.mods_dir();
                    let _ = std::fs::create_dir_all(&mods_dir);
                    let _ = open::that(&mods_dir);
                }

                if ui.add(
                    egui::Button::new("Обновить")
                        .fill(COLOR_BG_CARD)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(8.0)),
                ).clicked() {
                    self.refresh_installed_mods();
                }
            });
        });

        ui.add_space(14.0);

        if self.installed_mods.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(
                    RichText::new("В папке mods пока нет файлов.\nПерейдите в «Каталог модов», чтобы скачать их в один клик!")
                        .color(COLOR_TEXT_MUTED)
                        .size(14.0),
                );
            });
        } else {
            for mod_item in &self.installed_mods {
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&mod_item.filename).color(COLOR_TEXT_PRIMARY).strong().size(13.5));
                            ui.label(RichText::new(&mod_item.size_formatted).color(COLOR_TEXT_MUTED).size(12.0));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.add(
                                    egui::Button::new(RichText::new("Удалить").color(COLOR_RED).size(12.5).strong())
                                        .fill(COLOR_RED_BG)
                                        .stroke(Stroke::new(1.0_f32, COLOR_RED))
                                        .rounding(Rounding::same(6.0)),
                                ).clicked() {
                                    let _ = std::fs::remove_file(&mod_item.path);
                                }
                            });
                        });
                    });
                ui.add_space(6.0);
            }
        }
    }

    // ------------------------------------------------------------------------
    // TAB 5: SETTINGS
    // ------------------------------------------------------------------------
    fn render_settings_tab(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(COLOR_BG_CARD)
            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
            .rounding(Rounding::same(12.0))
            .inner_margin(Margin::same(24.0))
            .show(ui, |ui| {
                ui.label(RichText::new("Настройки Catrinth Launcher").size(20.0).strong().color(COLOR_TEXT_PRIMARY));
                ui.add_space(16.0);

                // Game Directory
                ui.label(RichText::new("Папка игры Minecraft (.minecraft):").strong().color(COLOR_TEXT_PRIMARY));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.config.game_dir).desired_width(460.0));
                    if ui.add(egui::Button::new("Обзор...").rounding(Rounding::same(6.0))).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.config.game_dir = folder.to_string_lossy().to_string();
                        }
                    }
                });

                ui.add_space(18.0);

                // RAM Sliders & Presets
                ui.label(RichText::new("Выделение оперативной памяти (ОЗУ):").strong().color(COLOR_TEXT_PRIMARY));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for gb in [2, 4, 6, 8, 12, 16] {
                        let mb = gb * 1024;
                        let is_active = self.config.ram_max_mb == mb;
                        let (bg, text_color) = if is_active {
                            (COLOR_PRIMARY, Color32::from_rgb(8, 20, 12))
                        } else {
                            (COLOR_BG_SURFACE, COLOR_TEXT_BODY)
                        };

                        let btn = egui::Button::new(
                            RichText::new(format!("{} GB", gb))
                                .color(text_color)
                                .strong(),
                        )
                        .fill(bg)
                        .rounding(Rounding::same(6.0));

                        if ui.add(btn).clicked() {
                            self.config.ram_max_mb = mb;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.add(
                    egui::Slider::new(&mut self.config.ram_max_mb, 1024..=32768)
                        .text("МБ Максимум (-Xmx)")
                        .step_by(512.0),
                );

                ui.add_space(18.0);

                // Java Runtime Path
                ui.label(RichText::new("Путь к исполняемому файлу Java:").strong().color(COLOR_TEXT_PRIMARY));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.config.java_path).desired_width(460.0));
                    if ui.add(egui::Button::new("Обзор...").rounding(Rounding::same(6.0))).clicked() {
                        if let Some(file) = rfd::FileDialog::new().pick_file() {
                            self.config.java_path = file.to_string_lossy().to_string();
                        }
                    }
                });
                ui.label(
                    RichText::new("Оставьте пустым для автоматического выбора Eclipse Temurin Java в папке runtime/.")
                        .color(COLOR_TEXT_MUTED)
                        .size(12.0),
                );

                ui.add_space(18.0);

                // JVM Arguments
                ui.label(RichText::new("Аргументы виртуальной машины JVM:").strong().color(COLOR_TEXT_PRIMARY));
                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut self.config.jvm_args).desired_width(580.0));

                ui.add_space(18.0);

                // Catrinth Proxy URL
                ui.label(RichText::new("Адрес сервера Catrinth Hub / Proxy:").strong().color(COLOR_TEXT_PRIMARY));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.config.api_proxy_url).desired_width(400.0));
                    if ui.add(egui::Button::new("Проверить пинг").rounding(Rounding::same(6.0))).clicked() {
                        let client = CatrinthClient::new(self.config.api_proxy_url.clone());
                        match client.check_health() {
                            Ok(ms) => self.status_connection = Some(format!("Подключено! Пинг: {} ms", ms)),
                            Err(e) => self.status_connection = Some(format!("Ошибка: {}", e)),
                        }
                    }
                });

                if let Some(msg) = &self.status_connection {
                    ui.add_space(4.0);
                    ui.label(RichText::new(msg).color(COLOR_PRIMARY).size(12.5).strong());
                }

                ui.add_space(24.0);

                // Save Button
                if ui.add(
                    egui::Button::new(
                        RichText::new("Сохранить настройки")
                            .color(Color32::from_rgb(8, 20, 12))
                            .size(14.0)
                            .strong(),
                    )
                    .fill(COLOR_PRIMARY)
                    .rounding(Rounding::same(8.0))
                    .min_size(vec2(220.0, 40.0)),
                ).clicked() {
                    let _ = self.config.save();
                    self.status_connection = Some("Настройки успешно сохранены!".to_string());
                }
            });
    }
}
