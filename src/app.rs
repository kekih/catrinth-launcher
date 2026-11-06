use crate::catrinth_api::{CatrinthClient, ModHit};
use crate::config::LauncherConfig;
use crate::launcher::{InstalledMod, MinecraftLauncher};
use crate::theme::{
    COLOR_BG_CARD, COLOR_BG_INPUT, COLOR_BG_MAIN, COLOR_BG_SURFACE, COLOR_BORDER,
    COLOR_PRIMARY, COLOR_PRIMARY_HOVER, COLOR_PURPLE, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use eframe::egui;
use egui::{vec2, Align, Align2, Color32, CornerRadius, Layout, RichText, Stroke, Ui};
use std::path::Path;

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
    status_message: Option<(String, bool)>, // (msg, is_error)

    // Mods Catalog State
    search_query: String,
    search_category: String,
    search_results: Vec<ModHit>,
    is_searching: bool,
    mod_status_message: Option<(String, bool)>,

    // Installed Mods State
    installed_mods: Vec<InstalledMod>,

    // Settings State
    status_connection: Option<String>,
}

impl CatrinthApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = LauncherConfig::load();
        let versions = MinecraftLauncher::get_installed_versions(&config);
        let mods = MinecraftLauncher::get_installed_mods(&config);

        let mut app = Self {
            config,
            active_tab: Tab::Play,
            available_versions: versions,
            status_message: None,
            search_query: String::new(),
            search_category: "mod".to_string(),
            search_results: Vec::new(),
            is_searching: false,
            mod_status_message: None,
            installed_mods: mods,
            status_connection: None,
        };

        // Initial search for popular mods
        app.perform_search();
        app
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
}

impl eframe::App for CatrinthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top Navigation Header
        egui::TopBottomPanel::top("top_header")
            .frame(
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .inner_margin(egui::Margin::symmetric(24, 16))
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
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| match self.active_tab {
                Tab::Play => self.render_play_tab(ui),
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
    .corner_radius(CornerRadius::same(6));

    if ui.add(btn).clicked() {
        on_click();
    }
}

impl CatrinthApp {
    // ------------------------------------------------------------------------
    // TAB 1: PLAY
    // ------------------------------------------------------------------------
    fn render_play_tab(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            // Hero Play Card
            egui::Frame::none()
                .fill(COLOR_BG_CARD)
                .stroke(Stroke::new(1.0, COLOR_BORDER))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(28))
                .show(ui, |ui| {
                    ui.set_max_width(700.0);

                    ui.label(
                        RichText::new("Minecraft готов к запуску")
                            .color(COLOR_TEXT_PRIMARY)
                            .size(24.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Запуск через независимый клиент на базе Rust")
                            .color(COLOR_TEXT_MUTED)
                            .size(13.0),
                    );

                    ui.add_space(20.0);

                    // Version & Username Grid
                    egui::Grid::new("play_grid")
                        .num_columns(2)
                        .spacing(vec2(16.0, 14.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Никнейм игрока:").strong());
                            ui.add(
                                egui::TextEdit::singleline(&mut self.config.username)
                                    .desired_width(320.0),
                            );
                            ui.end_row();

                            ui.label(RichText::new("Версия Minecraft:").strong());
                            egui::ComboBox::from_id_salt("version_select")
                                .selected_text(&self.config.selected_version)
                                .width(320.0)
                                .show_ui(ui, |ui| {
                                    for v in &self.available_versions {
                                        ui.selectable_value(
                                            &mut self.config.selected_version,
                                            v.clone(),
                                            v,
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

                    ui.add_space(24.0);

                    // Launch Button
                    let launch_btn = egui::Button::new(
                        RichText::new("▶  ИГРАТЬ В MINECRAFT")
                            .color(Color32::from_rgb(11, 14, 20))
                            .size(17.0)
                            .strong(),
                    )
                    .fill(COLOR_PRIMARY)
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(340.0, 50.0));

                    if ui.add(launch_btn).clicked() {
                        let _ = self.config.save();
                        match MinecraftLauncher::launch(&self.config, &self.config.selected_version)
                        {
                            Ok(msg) => self.status_message = Some((msg, false)),
                            Err(err) => self.status_message = Some((err, true)),
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
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    let search_input = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Поиск модов, плагинов, шейдеров в Catrinth...")
                            .desired_width(360.0),
                    );

                    if search_input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.perform_search();
                    }

                    egui::ComboBox::from_id_salt("cat_select")
                        .selected_text(match self.search_category.as_str() {
                            "mod" => "Моды (Forge / Fabric / NeoForge)",
                            "plugin" => "Плагины (Paper / Spigot)",
                            "shader" => "Шейдеры",
                            "resourcepack" => "Текстуры",
                            _ => "Все категории",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.search_category, "all".to_string(), "Все категории");
                            ui.selectable_value(&mut self.search_category, "mod".to_string(), "Моды");
                            ui.selectable_value(&mut self.search_category, "plugin".to_string(), "Плагины");
                            ui.selectable_value(&mut self.search_category, "shader".to_string(), "Шейдеры");
                            ui.selectable_value(&mut self.search_category, "resourcepack".to_string(), "Текстуры");
                        });

                    if ui.button("Найти").clicked() {
                        self.perform_search();
                    }
                });

                if let Some((msg, is_err)) = &self.mod_status_message {
                    ui.add_space(8.0);
                    let col = if *is_err { Color32::from_rgb(244, 63, 94) } else { COLOR_PRIMARY };
                    ui.label(RichText::new(msg).color(col).size(12.0));
                }
            });

        ui.add_space(14.0);

        // Mods Results Scroll Area
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.search_results.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("📦").size(32.0));
                    ui.label(RichText::new("Моды не найдены или введите запрос для поиска").color(COLOR_TEXT_MUTED));
                });
                return;
            }

            for hit in &self.search_results.clone() {
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0, COLOR_BORDER))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&hit.title).size(15.0).strong().color(Color32::WHITE));
                                    if let Some(author) = &hit.author {
                                        ui.label(RichText::new(format!("от {}", author)).color(COLOR_TEXT_MUTED).size(12.0));
                                    }
                                });

                                if let Some(desc) = &hit.description {
                                    ui.label(RichText::new(desc).color(COLOR_TEXT_MUTED).size(12.0));
                                }

                                ui.horizontal(|ui| {
                                    if let Some(dls) = hit.downloads {
                                        ui.label(RichText::new(format!("⬇ {} скачиваний", dls)).color(COLOR_PRIMARY).size(11.0));
                                    }
                                    if let Some(pt) = &hit.project_type {
                                        ui.label(RichText::new(pt.to_uppercase()).color(COLOR_PURPLE).size(11.0).strong());
                                    }
                                });
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let download_btn = egui::Button::new(
                                    RichText::new("📥 Скачать в mods/")
                                        .color(Color32::from_rgb(11, 14, 20))
                                        .size(12.0)
                                        .strong()
                                )
                                .fill(COLOR_PRIMARY)
                                .corner_radius(CornerRadius::same(6));

                                if ui.add(download_btn).clicked() {
                                    self.download_mod_action(hit);
                                }
                            });
                        });
                    });
                ui.add_space(8.0);
            }
        });
    }

    fn download_mod_action(&mut self, hit: &ModHit) {
        let client = CatrinthClient::new(self.config.api_proxy_url.clone());
        let id_or_slug = hit.slug.as_deref().or(hit.project_id.as_deref()).unwrap_or(&hit.title);

        match client.get_project_versions(id_or_slug) {
            Ok(versions) => {
                if let Some(latest) = versions.first() {
                    if let Some(primary_file) = latest.files.iter().find(|f| f.primary.unwrap_or(false)).or_else(|| latest.files.first()) {
                        let target_dir = self.config.mods_dir();
                        let target_path = target_dir.join(&primary_file.filename);

                        match client.download_file(&primary_file.url, &target_path) {
                            Ok(bytes) => {
                                self.mod_status_message = Some((
                                    format!("✓ Мод '{}' успешно скачан ({}) в папку mods!", primary_file.filename, crate::launcher::format_bytes(bytes)),
                                    false
                                ));
                                self.refresh_installed_mods();
                            }
                            Err(e) => {
                                self.mod_status_message = Some((format!("Ошибка скачивания: {}", e), true));
                            }
                        }
                    } else {
                        self.mod_status_message = Some(("У этой версии нет файлов для загрузки".to_string(), true));
                    }
                } else {
                    self.mod_status_message = Some(("У проекта нет доступных версий".to_string(), true));
                }
            }
            Err(e) => {
                self.mod_status_message = Some((format!("Ошибка получения версий: {}", e), true));
            }
        }
    }

    // ------------------------------------------------------------------------
    // TAB 3: INSTALLED MODS
    // ------------------------------------------------------------------------
    fn render_installed_mods_tab(&mut self, ui: &mut Ui) {
        // Controls Header
        egui::Frame::none()
            .fill(COLOR_BG_CARD)
            .stroke(Stroke::new(1.0, COLOR_BORDER))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Установленные моды: {}", self.installed_mods.len())).size(15.0).strong());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("📂 Открыть папку mods").clicked() {
                            let mods_dir = self.config.mods_dir();
                            let _ = std::fs::create_dir_all(&mods_dir);
                            let _ = open::that(&mods_dir);
                        }

                        if ui.button("🔄 Обновить список").clicked() {
                            self.refresh_installed_mods();
                        }
                    });
                });
            });

        ui.add_space(14.0);

        // Mods List
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.installed_mods.is_empty() {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("📂").size(32.0));
                    ui.label(RichText::new("В папке mods пока нет модов").color(COLOR_TEXT_MUTED));
                    ui.label(RichText::new("Перейдите во вкладку 'Каталог модов', чтобы установить их в 1 клик").color(COLOR_TEXT_MUTED).size(12.0));
                });
                return;
            }

            let mut to_delete = None;

            for (idx, m) in self.installed_mods.iter().enumerate() {
                egui::Frame::none()
                    .fill(COLOR_BG_CARD)
                    .stroke(Stroke::new(1.0, COLOR_BORDER))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("📄");
                            ui.label(RichText::new(&m.filename).size(13.0).strong().color(Color32::WHITE));
                            ui.label(RichText::new(&m.size_formatted).color(COLOR_TEXT_MUTED).size(12.0));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let del_btn = egui::Button::new(RichText::new("🗑 Удалить").color(Color32::from_rgb(244, 63, 94)).size(11.0));
                                if ui.add(del_btn).clicked() {
                                    to_delete = Some(idx);
                                }
                            });
                        });
                    });
                ui.add_space(6.0);
            }

            if let Some(idx) = to_delete {
                let m = &self.installed_mods[idx];
                let _ = std::fs::remove_file(&m.path);
                self.refresh_installed_mods();
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
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::same(24))
                .show(ui, |ui| {
                    ui.label(RichText::new("Настройки Catrinth Launcher").size(20.0).strong());
                    ui.add_space(14.0);

                    // --- 1. Game Directory ---
                    ui.label(RichText::new("Папка с игрой (.minecraft)").size(15.0).strong().color(COLOR_PRIMARY));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.config.game_dir).desired_width(420.0));

                        if ui.button("📁 Выбрать").clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                self.config.game_dir = folder.to_string_lossy().to_string();
                                self.available_versions = MinecraftLauncher::get_installed_versions(&self.config);
                                self.refresh_installed_mods();
                            }
                        }

                        if ui.button("По умолчанию").clicked() {
                            self.config.game_dir = LauncherConfig::default_minecraft_dir().to_string_lossy().to_string();
                        }

                        if ui.button("Открыть").clicked() {
                            let _ = open::that(&self.config.game_dir);
                        }
                    });

                    ui.add_space(18.0);

                    // --- 2. RAM Allocation ---
                    ui.label(RichText::new("Выделение оперативной памяти (RAM)").size(15.0).strong().color(COLOR_PRIMARY));
                    ui.label(RichText::new(format!(
                        "Текущее выделение: {} МБ ({:.1} ГБ)",
                        self.config.ram_max_mb,
                        self.config.ram_max_mb as f32 / 1024.0
                    )).color(COLOR_TEXT_MUTED));

                    ui.add(egui::Slider::new(&mut self.config.ram_max_mb, 1024..=16384).step_by(512.0).text("Макс. RAM (МБ)"));

                    ui.horizontal(|ui| {
                        ui.label("Быстрый выбор:");
                        if ui.button("2 ГБ").clicked() { self.config.ram_max_mb = 2048; }
                        if ui.button("4 ГБ").clicked() { self.config.ram_max_mb = 4096; }
                        if ui.button("6 ГБ").clicked() { self.config.ram_max_mb = 6144; }
                        if ui.button("8 ГБ").clicked() { self.config.ram_max_mb = 8192; }
                        if ui.button("12 ГБ").clicked() { self.config.ram_max_mb = 12288; }
                        if ui.button("16 ГБ").clicked() { self.config.ram_max_mb = 16384; }
                    });

                    ui.add_space(18.0);

                    // --- 3. Java Runtime ---
                    ui.label(RichText::new("Java & Аргументы запуска").size(15.0).strong().color(COLOR_PRIMARY));
                    ui.horizontal(|ui| {
                        ui.label("Путь к Java:");
                        ui.add(egui::TextEdit::singleline(&mut self.config.java_path).desired_width(340.0));
                        if ui.button("Выбрать файл").clicked() {
                            if let Some(file) = rfd::FileDialog::new().pick_file() {
                                self.config.java_path = file.to_string_lossy().to_string();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("JVM флаги:");
                        ui.add(egui::TextEdit::singleline(&mut self.config.jvm_args).desired_width(450.0));
                    });

                    ui.add_space(18.0);

                    // --- 4. Catrinth API Proxy ---
                    ui.label(RichText::new("Сервер Catrinth Hub").size(15.0).strong().color(COLOR_PRIMARY));
                    ui.horizontal(|ui| {
                        ui.label("URL прокси:");
                        ui.add(egui::TextEdit::singleline(&mut self.config.api_proxy_url).desired_width(320.0));

                        if ui.button("Проверить связь").clicked() {
                            let client = CatrinthClient::new(self.config.api_proxy_url.clone());
                            match client.check_health() {
                                Ok(msg) => self.status_connection = Some(format!("✓ {}", msg)),
                                Err(e) => self.status_connection = Some(format!("✗ {}", e)),
                            }
                        }
                    });

                    if let Some(conn) = &self.status_connection {
                        ui.label(RichText::new(conn).color(COLOR_PRIMARY).size(12.0));
                    }

                    ui.add_space(24.0);

                    // Save Button
                    if ui.add(
                        egui::Button::new(RichText::new("💾 Сохранить настройки").size(14.0).strong())
                            .fill(COLOR_PRIMARY)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(vec2(200.0, 36.0))
                    ).clicked() {
                        let _ = self.config.save();
                        self.status_connection = Some("✓ Настройки успешно сохранены!".to_string());
                    }
                });
        });
    }
}
