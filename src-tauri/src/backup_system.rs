use crate::settings_manager::AppSettings;
use chrono::{DateTime, Local};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

const BACKUPS_DIR_NAME: &str = "Backups";

#[derive(Serialize, Clone, Debug)]
pub struct BackupEntry {
    /// フォルダ名としても使用されるタイムスタンプ形式の文字列（例: "20231024_153000"）
    pub timestamp: String,
    /// ソートや比較に利用するための数値型タイムスタンプ（将来的な拡張用）
    pub timestamp_raw: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SubFolderState {
    /// 監視対象内のサブフォルダ名（カテゴリ名）
    pub name: String,
    /// ユーザーが設定した自由記述のメモ（meta.json から読み込まれる）
    pub memo: String,
    /// このフォルダに関連付けられたバックアップ履歴のリスト（新しい順）
    pub backups: Vec<BackupEntry>,
    /// 監視対象（Repo内の実体フォルダ）が現在存在するかどうかのフラグ
    pub source_exists: bool,
}

pub struct BackupSystem {
    app_handle: AppHandle,
    settings: Arc<Mutex<AppSettings>>,
    watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    // サブフォルダごとの最終更新時刻を保持し、短時間の連続した変更を1つのバックアップにまとめる（デバウンス用）
    debounce_map: Arc<Mutex<HashMap<String, DateTime<Local>>>>,
    // ファイル削除イベントを一時的に記録し、削除に伴う無関係なフォルダ更新イベントを無視するために使用
    delete_tracker: Arc<Mutex<HashMap<String, DateTime<Local>>>>,
    // リストア実行中に発生するファイルシステムイベントを無視するためのフラグ
    is_restoring: Arc<Mutex<bool>>,
}

impl BackupSystem {
    pub fn new(app_handle: AppHandle, settings: AppSettings) -> Self {
        // バックアップディレクトリが存在することを確認
        let exe_path = std::env::current_exe().unwrap_or(PathBuf::from("."));
        let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
        let backups_root = exe_dir.join(BACKUPS_DIR_NAME);
        let _ = fs::create_dir_all(&backups_root);

        Self {
            app_handle,
            settings: Arc::new(Mutex::new(settings)),
            watcher: Arc::new(Mutex::new(None)),
            debounce_map: Arc::new(Mutex::new(HashMap::new())),
            delete_tracker: Arc::new(Mutex::new(HashMap::new())),
            is_restoring: Arc::new(Mutex::new(false)),
        }
    }

    pub fn update_settings(&self, new_settings: AppSettings) {
        {
            let mut settings = self.settings.lock().unwrap();
            *settings = new_settings;
        }
        // 新しいパスでウォッチャーを再初期化
        self.start_watcher();
    }

    pub fn get_settings(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn start_watcher(&self) {
        let settings = self.settings.lock().unwrap();
        let path_str = &settings.repo_save_path;
        let path = Path::new(path_str);

        if !path.exists() {
            println!("リポジトリパスが存在しません: {}", path_str);
            return;
        }

        let app_handle = self.app_handle.clone();
        let debounce_map = self.debounce_map.clone();
        let delete_tracker = self.delete_tracker.clone();
        let settings_clone = self.settings.clone();
        let restoring_lock = self.is_restoring.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();

        // 指定されたパス配下のすべての変更（再帰的）を監視対象に登録
        match watcher.watch(path, RecursiveMode::Recursive) {
            Ok(_) => println!("監視を開始しました: {}", path_str),
            Err(e) => println!("パスの監視に失敗しました: {:?}", e),
        }

        // ウォッチャーを保持して破棄されないようにする
        let mut w = self.watcher.lock().unwrap();
        *w = Some(watcher);

        // イベントループを別スレッドで実行
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(res) => {
                        match res {
                            Ok(event) => {
                                // リストア実行中（自身によるファイル変更）は無視して無限ループを防ぐ
                                if *restoring_lock.lock().unwrap() {
                                    continue;
                                }

                                use notify::{event::RemoveKind, EventKind};
                                match event.kind {
                                    EventKind::Remove(RemoveKind::Folder) => {
                                        // 監視対象のフォルダ自体が削除された場合、フロントエンドの表示を更新
                                        println!("フォルダが削除されました。状態を更新します。");
                                        let exe_path = std::env::current_exe().unwrap_or_default();
                                        let backups_root =
                                            exe_path.parent().unwrap().join("Backups");
                                        Self::emit_state(&app_handle, &backups_root);

                                        // 削除イベント発生時に親フォルダから「更新」イベントも飛んでくるため、それを除外するためにマーク
                                        Self::mark_deletion(
                                            &event,
                                            &settings_clone,
                                            &delete_tracker,
                                        );
                                        continue;
                                    }
                                    EventKind::Remove(_) => {
                                        // 個別ファイルの削除イベントをマーク（後の修正イベント無視に使用）
                                        Self::mark_deletion(
                                            &event,
                                            &settings_clone,
                                            &delete_tracker,
                                        );
                                        continue;
                                    }
                                    _ => {}
                                }

                                // パスが存在するかチェック。削除イベントなどの場合はパスが存在しなくなっているため無視。
                                // ユーザー要望により「削除」はバックアップのトリガーから外している。
                                let mut exists = false;
                                for p in &event.paths {
                                    if p.exists() {
                                        exists = true;
                                        break;
                                    }
                                }
                                if !exists {
                                    continue;
                                }

                                // 有効な変更イベントをハンドリング
                                Self::handle_fs_event(
                                    event,
                                    &app_handle,
                                    &debounce_map,
                                    &delete_tracker,
                                    &settings_clone,
                                );
                            }
                            Err(e) => println!("監視エラー: {:?}", e),
                        }
                    }
                    Err(_) => break, // チャンネルがクローズされたらループ終了
                }
            }
        });
    }

    fn mark_deletion(
        event: &Event,
        settings_lock: &Arc<Mutex<AppSettings>>,
        delete_tracker: &Arc<Mutex<HashMap<String, DateTime<Local>>>>,
    ) {
        let settings = settings_lock.lock().unwrap();
        let repo_root = Path::new(&settings.repo_save_path);

        for path in &event.paths {
            if let Ok(rel_path) = path.strip_prefix(repo_root) {
                if let Some(first_comp) = rel_path.components().next() {
                    let subfolder_name = first_comp.as_os_str().to_string_lossy().to_string();
                    let mut tracker = delete_tracker.lock().unwrap();
                    tracker.insert(subfolder_name, Local::now());
                }
            }
        }
    }

    fn handle_fs_event(
        event: Event,
        app_handle: &AppHandle,
        debounce_map: &Arc<Mutex<HashMap<String, DateTime<Local>>>>,
        delete_tracker: &Arc<Mutex<HashMap<String, DateTime<Local>>>>,
        settings_lock: &Arc<Mutex<AppSettings>>,
    ) {
        // RepoSavePath のサブフォルダ内での変更に関心がある
        // event.paths に変更されたファイルが含まれる。
        // RepoSavePath のどの直下サブフォルダが変更されたかを特定する必要がある。

        let settings = settings_lock.lock().unwrap();
        let repo_root = Path::new(&settings.repo_save_path);

        let mut affected_subfolders = Vec::new();

        for path in event.paths {
            // Find relative path from repo_root
            if let Ok(rel_path) = path.strip_prefix(repo_root) {
                // If the path is just the root itself, ignore (or handle if files are there, but req said subfolders)
                if let Some(first_comp) = rel_path.components().next() {
                    let subfolder_name = first_comp.as_os_str().to_string_lossy().to_string();
                    if !affected_subfolders.contains(&subfolder_name) {
                        affected_subfolders.push(subfolder_name);
                    }
                }
            }
        }

        for folder in affected_subfolders {
            // CHECK: Was there a deletion recently?
            {
                let tracker = delete_tracker.lock().unwrap();
                if let Some(del_time) = tracker.get(&folder) {
                    let diff = Local::now().signed_duration_since(*del_time);
                    if diff.num_milliseconds() < 500 {
                        // 削除に伴う変更の可能性が高いため、この変更を無視
                        println!(
                            "最近の削除が原因と思われるため、{} の変更を無視しました",
                            folder
                        );
                        continue;
                    }
                }
            }

            let mut map = debounce_map.lock().unwrap();
            let now = Local::now();

            // 変更イベントが発生してから 300ms 待機し、その間に新たな変更がなければバックアップを実行
            // これにより、大量のファイルが短時間に連続して更新された際の負荷を抑えます

            map.insert(folder.clone(), now);

            let app_handle_clone = app_handle.clone();
            let settings_clone = settings_lock.clone();
            let debounce_map_clone = debounce_map.clone();
            let folder_clone = folder.clone();

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(300));

                let map = debounce_map_clone.lock().unwrap();
                if let Some(last_time) = map.get(&folder_clone) {
                    if *last_time == now {
                        // 条件合致！バックアップを実行
                        drop(map); // ロック解除
                        Self::perform_backup(&app_handle_clone, &settings_clone, &folder_clone);
                    }
                }
            });
        }
    }

    pub fn trigger_backup(&self, subfolder: &str) {
        Self::perform_backup(&self.app_handle, &self.settings, subfolder);
    }

    fn perform_backup(
        app_handle: &AppHandle,
        settings_lock: &Arc<Mutex<AppSettings>>,
        subfolder: &str,
    ) {
        println!("バックアップを実行中: {}", subfolder);
        let settings = settings_lock.lock().unwrap();
        let src_path = Path::new(&settings.repo_save_path).join(subfolder);

        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
        let backups_root = exe_dir.join(BACKUPS_DIR_NAME);

        let timestamp_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let dest_path = backups_root.join(subfolder).join(&timestamp_str);

        // コピー処理
        if let Err(e) = Self::copy_dir_recursive(&src_path, &dest_path) {
            println!("バックアップに失敗しました: {:?}", e);
            return;
        }

        // 世代制限の確認
        Self::enforce_generation_limit(&backups_root.join(subfolder), settings.max_generations);

        // Emit update
        Self::emit_state(app_handle, &backups_root);

        Self::send_notification(
            app_handle,
            "バックアップ作成",
            &format!("{} のバックアップを作成しました", subfolder),
        );
    }

    /// フォルダ構造を維持したまま、中身を再帰的にコピーする
    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }
        for entry in WalkDir::new(src) {
            let entry = entry?;
            let rel_path = entry.path().strip_prefix(src).unwrap();
            let dest_path = dst.join(rel_path);

            if entry.file_type().is_dir() {
                // ディレクトリなら作成
                fs::create_dir_all(dest_path)?;
            } else {
                // ファイルならコピー（既に存在する場合は上書き）
                fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }

    fn enforce_generation_limit(backup_folder_path: &Path, limit: usize) {
        // バックアップフォルダ内のディレクトリ（タイムスタンプ形式の名前）をリストアップ
        let Ok(entries) = fs::read_dir(backup_folder_path) else {
            return;
        };

        let mut backups: Vec<(PathBuf, String)> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| (e.path(), e.file_name().to_string_lossy().to_string()))
            .collect();

        // 名前（タイムスタンプ）で昇順ソート（古いものが最初に来る）
        backups.sort_by(|a, b| a.1.cmp(&b.1));

        // 保持件数を超えている場合、古い順に削除
        if backups.len() > limit {
            let to_remove = backups.len() - limit;
            for i in 0..to_remove {
                let _ = fs::remove_dir_all(&backups[i].0);
            }
        }
    }

    pub fn emit_state(app_handle: &AppHandle, backups_root: &Path) {
        // 設定を逐次リロードして、現在の監視パス（repo_save_path）に基づく状態を特定する
        // NOTE: emit_state 自体は static な文脈で呼ばれることが多いため、SettingsManager を介してパスを取得
        let sm = crate::settings_manager::SettingsManager::new();
        let settings = sm.load();

        let repo_root = Path::new(&settings.repo_save_path);

        // 全バックアップと Repo 内の最新状態を統合してベクトル形式で取得
        let state = Self::get_all_state(backups_root, repo_root);
        // 全フロントエンド（および通知ウィンドウ）へ状態をブロードキャスト
        let _ = app_handle.emit("backups-state", state);
    }

    pub fn get_all_state(backups_root: &Path, repo_root: &Path) -> Vec<SubFolderState> {
        let mut results = HashMap::new();

        // 1. バックアップ済みフォルダをスキャン
        if backups_root.exists() {
            if let Ok(subfolders) = fs::read_dir(backups_root) {
                for entry in subfolders.filter_map(|e| e.ok()) {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();

                    // meta.json を読み込み
                    let meta_path = entry.path().join("meta.json");
                    let memo = if meta_path.exists() {
                        let content = fs::read_to_string(&meta_path).unwrap_or_default();
                        let json: serde_json::Value =
                            serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
                        json["memo"].as_str().unwrap_or("").to_string()
                    } else {
                        String::new()
                    };

                    // バックアップ一覧を取得
                    let mut backups = Vec::new();
                    if let Ok(bk_entries) = fs::read_dir(entry.path()) {
                        for bk in bk_entries.filter_map(|e| e.ok()) {
                            let fname = bk.file_name().to_string_lossy().to_string();
                            if fname == "meta.json" {
                                continue;
                            }
                            if bk.path().is_dir() {
                                backups.push(BackupEntry {
                                    timestamp: fname.clone(),
                                    timestamp_raw: 0,
                                });
                            }
                        }
                    }
                    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // 新しい順

                    // ソースフォルダが存在するか確認
                    let source_path = repo_root.join(&name);
                    let source_exists = source_path.exists();

                    results.insert(
                        name.clone(),
                        SubFolderState {
                            name,
                            memo,
                            backups,
                            source_exists,
                        },
                    );
                }
            }
        }

        // 2. バックアップされていないフォルダを Repo からスキャン
        if repo_root.exists() {
            if let Ok(entries) = fs::read_dir(repo_root) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();

                    if !results.contains_key(&name) {
                        // バックアップがまだ存在しないフォルダもリストに含めることで、
                        // ユーザーが手動バックアップを実行したり、監視の存在を認識したりできるようにします。
                        results.insert(
                            name.clone(),
                            SubFolderState {
                                name: name.clone(),
                                memo: String::new(),
                                backups: Vec::new(),
                                source_exists: true,
                            },
                        );
                    }
                }
            }
        }

        let mut list: Vec<SubFolderState> = results.into_values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn save_memo(backups_root: &Path, subfolder: &str, memo: &str) {
        let folder_path = backups_root.join(subfolder);
        // バックアップがまだない場合でもメモを保持できるよう、親ディレクトリを作成します。
        let _ = fs::create_dir_all(&folder_path);

        let meta_path = folder_path.join("meta.json");
        let data = serde_json::json!({ "memo": memo });
        let _ = fs::write(meta_path, data.to_string());
    }

    pub fn delete_backup(
        app_handle: &AppHandle,
        backups_root: &Path,
        subfolder: &str,
        timestamp: &str,
    ) {
        let target = backups_root.join(subfolder).join(timestamp);
        if target.exists() {
            let _ = fs::remove_dir_all(target);
            Self::send_notification(
                app_handle,
                "バックアップ削除",
                &format!(
                    "{} のバックアップ（{}）を削除しました",
                    subfolder, timestamp
                ),
            );
        }

        // 空（meta以外）になったらフォルダを削除
        let folder_path = backups_root.join(subfolder);
        let mut has_backups = false;
        if let Ok(entries) = fs::read_dir(&folder_path) {
            for e in entries.filter_map(|x| x.ok()) {
                if e.file_name().to_string_lossy() != "meta.json" && e.path().is_dir() {
                    has_backups = true;
                    break;
                }
            }
        }

        if !has_backups {
            let _ = fs::remove_dir_all(folder_path);
        }
    }

    pub fn delete_subfolder(app_handle: &AppHandle, backups_root: &Path, subfolder: &str) {
        let folder_path = backups_root.join(subfolder);
        if folder_path.exists() {
            let _ = fs::remove_dir_all(folder_path);
            Self::send_notification(
                app_handle,
                "全バックアップ削除",
                &format!("{} のすべてのバックアップを削除しました", subfolder),
            );
        }
    }

    pub fn restore_backup(
        &self,
        settings: &AppSettings,
        backups_root: &Path,
        subfolder: &str,
        timestamp: &str,
    ) {
        // リストア中フラグを設定
        {
            let mut lock = self.is_restoring.lock().unwrap();
            *lock = true;
        }

        let src = backups_root.join(subfolder).join(timestamp);
        let dest = Path::new(&settings.repo_save_path).join(subfolder);

        if src.exists() {
            if dest.exists() {
                let _ = fs::remove_dir_all(&dest);
            }
            if let Ok(_) = Self::copy_dir_recursive(&src, &dest) {
                Self::send_notification(
                    &self.app_handle,
                    "リストア完了",
                    &format!("{} を {} の時点にリストアしました", subfolder, timestamp),
                );
            }
        }

        thread::sleep(Duration::from_millis(1000));

        {
            let mut lock = self.is_restoring.lock().unwrap();
            *lock = false;
        }

        // リストア後に source_exists を更新するため状態をリフレッシュ
        let exe_path = std::env::current_exe().unwrap_or_default();
        let backups_root = exe_path.parent().unwrap().join("Backups");
        Self::emit_state(&self.app_handle, &backups_root);
    }

    pub fn send_notification(app: &AppHandle, title: &str, body: &str) {
        println!("[NOTIFICATION] 通知を送信中: {} - {}", title, body);
        // 通知ウィンドウにイベントを送信
        // ペイロード: { title, body, type } - 現在は簡略化
        // 通知の種類（アイコン/色）を、タイトルに含まれるキーワードに基づいて判定
        let type_str = if title.contains("バックアップ") {
            "backup"
        } else if title.contains("リストア") {
            "restore"
        } else if title.contains("削除") {
            "delete"
        } else {
            "success"
        };

        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "type": type_str
        });
        // Tauri v2 の AppHandle.emit は、デフォルトで全ウィンドウにブロードキャストされます。
        // 個別ウィンドウへの emit を併用すると重複受信の原因となるため、全体放送のみを行います。
        let _ = app.emit("show-notification", payload);
    }
}
