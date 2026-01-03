use ini::Ini;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = "settings.ini";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub repo_save_path: String,
    #[serde(default = "default_theme")]
    pub theme: String, // "dark", "light", "system"
    #[serde(default = "default_max_generations")]
    pub max_generations: usize,
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_max_generations() -> usize {
    10
}

impl Default for AppSettings {
    fn default() -> Self {
        // 開発元の仕様に基づくデフォルトのセーブデータパス
        // %USERPROFILE%/AppData/LocalLow/semiwork/Repo/saves を動的に解決
        let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
        let default_path = Path::new(&user_profile)
            .join("AppData")
            .join("LocalLow")
            .join("semiwork")
            .join("Repo")
            .join("saves");

        Self {
            repo_save_path: default_path.to_string_lossy().to_string(), // 初期パス
            theme: "system".to_string(),                                // 初期テーマ
            max_generations: 10,                                        // 初期保持世代数
        }
    }
}

pub struct SettingsManager {
    file_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> Self {
        // In dev, use current dir. In production, use exe dir.
        // For simplicity in Tauri v2, we can just use std::env::current_exe() parent or standard dirs.
        // The plan said "app executable directory".
        let exe_path = std::env::current_exe().ok();
        let dir = match exe_path {
            Some(path) => path.parent().unwrap_or(Path::new(".")).to_path_buf(),
            None => PathBuf::from("."),
        };
        let file_path = dir.join(SETTINGS_FILE);

        Self { file_path }
    }

    pub fn load(&self) -> AppSettings {
        // 設定ファイルが存在しない場合は、デフォルト値を生成して保存
        if !self.file_path.exists() {
            let defaults = AppSettings::default();
            self.save(&defaults);
            return defaults;
        }

        // INIファイルを読み込み、構造体へパース
        match fs::read_to_string(&self.file_path) {
            Ok(content) => {
                let settings: AppSettings = Ini::load_from_str(&content)
                    .ok()
                    .and_then(|ini| {
                        let section = ini.section(Some("Settings"))?;
                        Some(AppSettings {
                            repo_save_path: section.get("repo_save_path")?.to_string(),
                            theme: section.get("theme").unwrap_or("system").to_string(),
                            max_generations: section
                                .get("max_generations")
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(10),
                        })
                    })
                    .unwrap_or_else(|| AppSettings::default()); // 失敗時はデフォルト値を返す
                settings
            }
            Err(_) => AppSettings::default(),
        }
    }

    pub fn save(&self, settings: &AppSettings) {
        // 設定内容を INI フォーマットでファイルへ書き出し
        let mut ini = Ini::new();
        ini.with_section(Some("Settings"))
            .set("repo_save_path", &settings.repo_save_path)
            .set("theme", &settings.theme)
            .set("max_generations", settings.max_generations.to_string());

        let _ = ini.write_to_file(&self.file_path);
    }
}
