use crate::catrinth_api::{CatrinthClient, ModHit};
use crate::config::LauncherConfig;
use crate::launcher::{InstalledMod, MinecraftLauncher};
use crate::mojang_api::{DownloadProgress, MojangClient, VersionManifest};
use crate::theme::{
    COLOR_BG_CARD, COLOR_BG_MAIN, COLOR_BG_SURFACE, COLOR_BORDER, COLOR_PRIMARY, COLOR_PURPLE,
    COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use eframe::egui;
use egui::{vec2, Align, Color32, Layout, Margin, RichText, Rounding, Stroke, Ui};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Play,
    ModsCatalog,
    InstalledMods,
    Settings,
}

pub struct CatrinthApp {
    config: LauncherConfig,
    active_tab: Tab,

    // Play Tab State
    available_versions: Vec<String>,
    show_snapshots: bool,
    status_message: Option<(String, bool)>, // (msg, is_error)

    // Download & Launch State
    download_progress: Arc<Mutex<DownloadProgress>>,
    is_downloading: bool,
    manifest_cache: Arc<Mutex<Option<VersionManifest>>>,

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
        let config = LauncherConfig::load();
        let mut versions = MinecraftLauncher::get_installed_versions(&config);
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
            show_snapshots: false,
            status_message: None,
            download_progress: Arc::new(Mutex::new(DownloadProgress::default())),
            is_downloading: false,
            manifest_cache: manifest_slot,
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
        let mut list = Vec::new();

        // 1. Locally installed versions always first
        let installed = MinecraftLauncher::get_installed_versions(&self.config);
        for v in &installed {
            if !list.contains(v) {
                list.push(v.clone());
            }
        }

        // 2. Official release versions from Mojang manifest
        for v in &manifest.versions {
            if v.version_type == "release"
                || (self.show_snapshots && v.version_type == "snapshot")
            {
                if !list.contains(&v.id) {
                    list.push(v.id.clone());
                }
            }
        }

        if !list.is_empty() {
            self.available_versions = list;
        }
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

    fn start_download_and_launch(&mut self, version: String, ctx: egui::Context) {
        self.is_downloading = true;
        self.status_message = None;

        let progress_arc = Arc::clone(&self.download_progress);
        {
            let mut p = progress_arc.lock().unwrap();
            *p = DownloadProgress {
                stage: format!("Подготовка к загрузке Minecraft {}...", version),
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

            // 2. Version Details
            {
                let mut p = progress_arc.lock().unwrap();
                p.stage = format!("Загрузка описания версии {}...", version);
                p.percent = 0.10;
            }
            ctx.request_repaint();

            let details = match mojang.fetch_version_details(&game_dir, &manifest, &version) {
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

            // 6. Launch!
            {
                let mut p = progress_arc.lock().unwrap();
                p.stage = "Все файлы готовы! Запуск Minecraft...".to_string();
                p.percent = 1.0;
            }
            ctx.request_repaint();

            match MinecraftLauncher::launch(&config_clone, &version) {
                Ok(msg) => {
                    let mut p = progress_arc.lock().unwrap();
                    p.message = msg;
                    p.is_done = true;
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
}

impl eframe::App for CatrinthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for fetched Mojang version manifest (safe release of lock)
        let manifest_opt = self.manifest_cache.try_lock().ok().and_then(|mut s| s.take());
        if let Some(manifest) = manifest_opt {
            self.apply_version_manifest(&manifest);
        }

        // Check download worker status
        if self.is_downloading {
            ctx.request_repaint();
            let (done, err, success) = {
                let p = self.download_progress.lock().unwrap();
                (p.is_done, p.error.clone(), p.message.clone())
            };

            if done {
                self.is_downloading = false;
                if let Some(e) = err {
                    self.status_message = Some((e, true));
                } else if !success.is_empty() {
                    self.status_message = Some((success, false));
                }
                self.available_versions =
                    MinecraftLauncher::get_installed_versions(&self.config);
            }
        }

        // Top Navigation Header
        egui::TopBottomPanel::top("top_header")
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .inner_margin(Margin {
                        left: 24.0,
                        right: 24.0,
                        top: 16.0,
                        bottom: 16.0,
                    })
                    .stroke(Stroke::new(1.0, COLOR_BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo & App Name
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("❖").color(COLOR_PRIMARY).size(22.0).strong());
                        ui.label(
                            RichText::new("CATRINTH")
                                .color(COLOR_TEXT_PRIMARY)
                                .size(18.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("LAUNCHER")
                                .color(COLOR_PRIMARY)
                                .size(18.0)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("RUST")
                                .color(COLOR_PURPLE)
                                .size(10.0)
                                .strong(),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Quick RAM pill
                        let ram_gb = self.config.ram_max_mb as f32 / 1024.0;
                        ui.label(
                            RichText::new(format!("⚡ {:.1} GB ОЗУ", ram_gb))
                                .color(COLOR_PRIMARY)
                                .size(12.0),
                        );

                        // User Pill
                        ui.label(
                            RichText::new(format!("👤 {}", self.config.username))
                                .color(COLOR_TEXT_PRIMARY)
                                .size(12.0)
                                .strong(),
                        );
                    });
                });

                ui.add_space(10.0);

                // Navigation Tabs Bar
                ui.horizontal(|ui| {
                    tab_button(ui, "🎮 Запуск игры", self.active_tab == Tab::Play, || {
                        self.active_tab = Tab::Play;
                    });
                    tab_button(
                        ui,
                        "📦 Каталог модов Catrinth",
                        self.active_tab == Tab::ModsCatalog,
                        || {
                            self.active_tab = Tab::ModsCatalog;
                        },
                    );
                    tab_button(
                        ui,
                        &format!("📁 Моды игры ({})", self.installed_mods.len()),
                        self.active_tab == Tab::InstalledMods,
                        || {
                            self.active_tab = Tab::InstalledMods;
                            self.refresh_installed_mods();
                        },
                    );
                    tab_button(ui, "⚙ Настройки", self.active_tab == Tab::Settings, || {
                        self.active_tab = Tab::Settings;
                    });
                });
            });

        // Main Content Area
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_MAIN)
                    .inner_margin(Margin::same(24.0)),
            )
            .show(ctx, |ui| match self.active_tab {
                Tab::Play => self.render_play_tab(ui, ctx),
                Tab::ModsCatalog => self.render_mods_tab(ui),
                Tab::InstalledMods => self.render_installed_mods_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
            });
    }
}

fn tab_button(ui: &mut Ui, label: &str, active: bool, mut on_click: impl FnMut()) {
    let (bg, text_color) = if active {
        (COLOR_PRIMARY, Color32::from_rgb(11, 14, 20))
    } else {
        (COLOR_BG_SURFACE, COLOR_TEXT_MUTED)
    };

    let btn = egui::Button::new(
        RichText::new(label)
            .color(text_color)
            .size(13.0)
            .strong(),
    )
    .fill(bg)
    .rounding(Rounding::same(6.0));

    if ui.add(btn).clicked() {
        on_click();
    }
}

impl CatrinthApp {
    // ------------------------------------------------------------------------
    // TAB 1: PLAY & DOWNLOAD
    // ------------------------------------------------------------------------
    fn render_play_tab(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.vertical_centered(|ui| {
            // Hero Play Card
            egui::Frame::none()
                .fill(COLOR_BG_CARD)
                .stroke(Stroke::new(1.0, COLOR_BORDER))
                .rounding(Rounding::same(12.0))
                .inner_margin(Margin::same(28.0))
                .show(ui, |ui| {
                    ui.set_max_width(700.0);

                    ui.label(
                        RichText::new("Minecraft готов к запуску")
                            .color(COLOR_TEXT_PRIMARY)
                            .size(24.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Нативный лаунчер Minecraft на Rust с автозагрузкой с piston-meta.mojang.com")
                            .color(COLOR_TEXT_MUTED)
                            .size(13.0),
                    );

                    ui.add_space(20.0);

                    let is_installed = MinecraftLauncher::is_version_installed(
                        &self.config,
                        &self.config.selected_version,
                    );

                    // Version & Username Grid
                    egui::Grid::new("play_grid")
                        .num_columns(2)
                        .spacing(vec2(16.0, 14.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Никнейм игрока:").strong());
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.username)
                                    .desired_width(340.0),
                            );
                            ui.end_row();

                            ui.label(RichText::new("Версия Minecraft:").strong());
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_source("version_select")
                                    .selected_text(&self.config.selected_version)
                                    .width(240.0)
                                    .show_ui(ui, |ui| {
                                        for v in &self.available_versions {
                                            let installed_tag = if MinecraftLauncher::is_version_installed(&self.config, v) {
                                                " [Установлена]"
                                            } else {
                                                ""
                                            };
                                            let label = format!("{}{}", v, installed_tag);
                                            ui.selectable_value(
                                                &mut self.config.selected_version,
                                                v.clone(),
                                                label,
                                            );
                                        }
                                    });

                                // Status Badge
                                if is_installed {
                                    ui.label(
                                        RichText::new("✅ Установлена")
                                            .color(COLOR_PRIMARY)
                                            .strong()
                                            .size(12.0),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new("⬇ Требуется скачивание")
                                            .color(COLOR_PURPLE)
                                            .size(12.0),
                                    );
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new("Папка игры:").strong());
                            ui.label(
                                RichText::new(&self.config.game_dir)
                                    .color(COLOR_TEXT_MUTED)
                                    .size(12.0),
                            );
                            ui.end_row();

                            ui.label(RichText::new("Выделение памяти:").strong());
                            ui.label(
                                RichText::new(format!(
                                    "{} МБ (макс. {:.1} ГБ)",
                                    self.config.ram_max_mb,
                                    self.config.ram_max_mb as f32 / 1024.0
                                ))
                                .color(COLOR_PRIMARY)
                                .strong(),
                            );
                            ui.end_row();
                        });

                    ui.add_space(20.0);

                    // Download Progress Display
                    if self.is_downloading {
                        egui::Frame::none()
                            .fill(COLOR_BG_SURFACE)
                            .stroke(Stroke::new(1.0, COLOR_PRIMARY))
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::same(16.0))
                            .show(ui, |ui| {
                                ui.set_max_width(640.0);
                                let p = self.download_progress.lock().unwrap().clone();
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("📦").size(18.0));
                                    ui.label(
                                        RichText::new(&p.stage)
                                            .color(COLOR_PRIMARY)
                                            .strong()
                                            .size(14.0),
                                    );
                                });

                                ui.add_space(8.0);
                                ui.add(
                                    egui::ProgressBar::new(p.percent)
                                        .show_percentage()
                                        .animate(true),
                                );

                                if !p.message.is_empty() {
                                    ui.add_space(6.0);
                                    ui.label(
                                        RichText::new(&p.message)
                                            .color(COLOR_TEXT_MUTED)
                                            .size(12.0),
                                    );
                                }
                            });
                        ui.add_space(14.0);
                    } else {
                        // Action Buttons
                        let ver = self.config.selected_version.clone();
                        let ctx_clone = ctx.clone();

                        if is_installed {
                            // Already installed: Launch or Re-check
                            ui.horizontal(|ui| {
                                let launch_btn = egui::Button::new(
                                    RichText::new("▶  ИГРАТЬ В MINECRAFT")
                                        .color(Color32::from_rgb(11, 14, 20))
                                        .size(16.0)
                                        .strong(),
                                )
                                .fill(COLOR_PRIMARY)
                                .rounding(Rounding::same(8.0))
                                .min_size(vec2(280.0, 48.0));

                                if ui.add(launch_btn).clicked() {
                                    let _ = self.config.save();
                                    match MinecraftLauncher::launch(&self.config, &ver) {
                                        Ok(msg) => self.status_message = Some((msg, false)),
                                        Err(err) => self.status_message = Some((err, true)),
                                    }
                                }

                                let update_btn = egui::Button::new(
                                    RichText::new("🔄 Обновить файлы")
                                        .color(COLOR_TEXT_PRIMARY)
                                        .size(13.0),
                                )
                                .fill(COLOR_BG_SURFACE)
                                .rounding(Rounding::same(8.0))
                                .min_size(vec2(160.0, 48.0));

                                if ui.add(update_btn).clicked() {
                                    let _ = self.config.save();
                                    self.start_download_and_launch(ver.clone(), ctx_clone);
                                }
                            });
                        } else {
                            // Not installed: Download and Launch
                            let dl_btn = egui::Button::new(
                                RichText::new(format!("⬇  СКАЧАТЬ И ЗАПУСТИТЬ MINECRAFT {}", ver))
                                    .color(Color32::from_rgb(11, 14, 20))
                                    .size(16.0)
                                    .strong(),
                            )
                            .fill(COLOR_PRIMARY)
                            .rounding(Rounding::same(8.0))
                            .min_size(vec2(400.0, 48.0));

                            if ui.add(dl_btn).clicked() {
                                let _ = self.config.save();
                                self.start_download_and_launch(ver.clone(), ctx_clone);
                            }
                        }
                    }

                    // Status Message Display
                    if let Some((msg, is_err)) = &self.status_message {
                        ui.add_space(14.0);
                        let col = if *is_err {
                            Color32::from_rgb(244, 63, 94)
                        } else {
                            COLOR_PRIMARY
                        };
                        ui.label(RichText::new(msg).color(col).size(13.0).strong());
                    }
                });
        });
    }

    // ------------------------------------------------------------------------
    // TAB 2: CATRINTH MODS CATALOG
    // ------------------------------------------------------------------------
    fn render_mods_tab(&mut self, ui: &mut Ui) {
        // Search & Filters Header
        egui::Frame::none()
            .fill(COLOR_BG_CARD)
            .stroke(Stroke::new(1.0, COLOR_BORDER))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(14.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    let edit = egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Поиск модов, шейдеров, датапаков из Catrinth...")
                        .desired_width(380.0);

                    if ui.add(edit).lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.perform_search();
                    }

                    egui::ComboBox::from_id_source("cat_select")
                        .selected_text(match self.search_category.as_str() {
                            "mod" => "Моды (Fabric / Forge)",
                            "shader" => "Шейдеры",
                            "resourcepack" => "Текстур-паки",
                            "datapack" => "Датапаки",
                            _ => "Все категории",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.search_category, "mod".into(), "Моды (Fabric / Forge)");
                            ui.selectable_value(&mut self.search_category, "shader".into(), "Шейдеры");
                            ui.selectable_value(&mut self.search_category, "resourcepack".into(), "Текстур-паки");
                            ui.selectable_value(&mut self.search_category, "datapack".into(), "Датапаки");
                        });

                    if ui.add(egui::Button::new("Найти").fill(COLOR_PRIMARY).rounding(Rounding::same(6.0))).clicked() {
                        self.perform_search();
                    }
                });
            });

        ui.add_space(14.0);

        // Status / Error message
        if let Some((msg, is_err)) = &self.mod_status_message {
            let col = if *is_err {
                Color32::from_rgb(244, 63, 94)
            } else {
                COLOR_PRIMARY
            };
            ui.label(RichText::new(msg).color(col).strong());
            ui.add_space(8.0);
        }

        // Search Results List
        let mut download_slug_action: Option<String> = None;
        let hits = self.search_results.clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if hits.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
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
                            .rounding(Rounding::same(8.0))
                            .inner_margin(Margin::same(16.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Main Info
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(title)
                                                    .color(COLOR_TEXT_PRIMARY)
                                                    .size(16.0)
                                                    .strong(),
                                            );
                                            ui.label(
                                                RichText::new(format!("[{}]", p_type))
                                                    .color(COLOR_PURPLE)
                                                    .size(11.0),
                                            );
                                            ui.label(
                                                RichText::new(format!("от {}", author))
                                                    .color(COLOR_TEXT_MUTED)
                                                    .size(12.0),
                                            );
                                        });

                                        if !desc.is_empty() {
                                            ui.add_space(4.0);
                                            ui.label(
                                                RichText::new(desc)
                                                    .color(COLOR_TEXT_MUTED)
                                                    .size(13.0),
                                            );
                                        }

                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("⬇ {} скачиваний", dl_count))
                                                    .color(COLOR_TEXT_MUTED)
                                                    .size(11.0),
                                            );
                                        });
                                    });

                                    // Right Action: Download Mod
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let download_btn = egui::Button::new(
                                            RichText::new("⬇ Скачать")
                                                .color(Color32::from_rgb(11, 14, 20))
                                                .strong(),
                                        )
                                        .fill(COLOR_PRIMARY)
                                        .rounding(Rounding::same(6.0));

                                        if ui.add(download_btn).clicked() {
                                            download_slug_action = Some(slug.to_string());
                                        }
                                    });
                                });
                            });
                        ui.add_space(8.0);
                    }
                }
            });

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
                                        format!("Мод {} успешно сохранён в mods!", file.filename),
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
    // TAB 3: INSTALLED MODS
    // ------------------------------------------------------------------------
    fn render_installed_mods_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Установленные моды в папке ({})", self.installed_mods.len()))
                    .size(18.0)
                    .strong(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(egui::Button::new("📂 Открыть папку mods").rounding(Rounding::same(6.0))).clicked() {
                    let mods_dir = self.config.mods_dir();
                    let _ = std::fs::create_dir_all(&mods_dir);
                    let _ = open::that(&mods_dir);
                }

                if ui.add(egui::Button::new("🔄 Обновить список").rounding(Rounding::same(6.0))).clicked() {
                    self.refresh_installed_mods();
                }
            });
        });

        ui.add_space(14.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.installed_mods.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            RichText::new("В папке mods пока нет файлов.\nПерейдите в «Каталог модов Catrinth», чтобы скачать их в один клик!")
                                .color(COLOR_TEXT_MUTED)
                                .size(14.0),
                        );
                    });
                } else {
                    for mod_item in &self.installed_mods {
                        egui::Frame::none()
                            .fill(COLOR_BG_CARD)
                            .stroke(Stroke::new(1.0, COLOR_BORDER))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("📄");
                                    ui.label(RichText::new(&mod_item.filename).color(COLOR_TEXT_PRIMARY).strong());
                                    ui.label(RichText::new(&mod_item.size_formatted).color(COLOR_TEXT_MUTED).size(12.0));

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.add(egui::Button::new("🗑 Удалить").rounding(Rounding::same(4.0))).clicked() {
                                            let _ = std::fs::remove_file(&mod_item.path);
                                        }
                                    });
                                });
                            });
                        ui.add_space(6.0);
                    }
                }
            });
    }

    // ------------------------------------------------------------------------
    // TAB 4: SETTINGS
    // ------------------------------------------------------------------------
    fn render_settings_tab(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Frame::none()
                .fill(COLOR_BG_CARD)
                .stroke(Stroke::new(1.0, COLOR_BORDER))
                .rounding(Rounding::same(12.0))
                .inner_margin(Margin::same(24.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("Настройки Catrinth Launcher").size(20.0).strong());
                    ui.add_space(14.0);

                    // Game Directory
                    ui.label(RichText::new("Папка игры Minecraft (.minecraft):").strong());
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.config.game_dir).desired_width(480.0));
                        if ui.add(egui::Button::new("📁 Обзор...").rounding(Rounding::same(6.0))).clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                self.config.game_dir = folder.to_string_lossy().to_string();
                            }
                        }
                    });

                    ui.add_space(16.0);

                    // RAM Sliders & Presets
                    ui.label(RichText::new("Выделение оперативной памяти (ОЗУ):").strong());
                    ui.horizontal(|ui| {
                        for gb in [2, 4, 6, 8, 12, 16] {
                            let mb = gb * 1024;
                            let is_active = self.config.ram_max_mb == mb;
                            let btn = egui::Button::new(format!("{} GB", gb))
                                .fill(if is_active { COLOR_PRIMARY } else { COLOR_BG_SURFACE })
                                .rounding(Rounding::same(4.0));
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

                    ui.add_space(16.0);

                    // Java Runtime Path
                    ui.label(RichText::new("Путь к Java (javaw.exe / java):").strong());
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.config.java_path).desired_width(480.0));
                        if ui.add(egui::Button::new("🔍 Обзор...").rounding(Rounding::same(6.0))).clicked() {
                            if let Some(file) = rfd::FileDialog::new().pick_file() {
                                self.config.java_path = file.to_string_lossy().to_string();
                            }
                        }
                    });
                    ui.label(
                        RichText::new("Оставьте пустым для автопоиска Java в системе (рекомендуется Java 21).")
                            .color(COLOR_TEXT_MUTED)
                            .size(11.0),
                    );

                    ui.add_space(16.0);

                    // JVM Arguments
                    ui.label(RichText::new("Аргументы JVM (Java Virtual Machine):").strong());
                    ui.add(egui::TextEdit::singleline(&mut self.config.jvm_args).desired_width(580.0));

                    ui.add_space(16.0);

                    // Catrinth Proxy URL
                    ui.label(RichText::new("Адрес сервера Catrinth Hub / Proxy:").strong());
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.config.api_proxy_url).desired_width(400.0));
                        if ui.add(egui::Button::new("📡 Проверить связь").rounding(Rounding::same(6.0))).clicked() {
                            let client = CatrinthClient::new(self.config.api_proxy_url.clone());
                            match client.check_health() {
                                Ok(ms) => self.status_connection = Some(format!("Подключено! Пинг: {} ms", ms)),
                                Err(e) => self.status_connection = Some(format!("Ошибка: {}", e)),
                            }
                        }
                    });

                    if let Some(msg) = &self.status_connection {
                        ui.label(RichText::new(msg).color(COLOR_PRIMARY).size(12.0));
                    }

                    ui.add_space(24.0);

                    // Save Button
                    if ui.add(
                        egui::Button::new(RichText::new("💾 Сохранить настройки").size(14.0).strong())
                            .fill(COLOR_PRIMARY)
                            .rounding(Rounding::same(6.0))
                            .min_size(vec2(200.0, 36.0)),
                    ).clicked() {
                        let _ = self.config.save();
                        self.status_connection = Some("Настройки успешно сохранены!".to_string());
                    }
                });
        });
    }
}
