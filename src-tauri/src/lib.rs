mod backup_system;
mod settings_manager;

use backup_system::BackupSystem;
use settings_manager::SettingsManager;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State,
};

#[tauri::command]
fn initialize_app(app: tauri::AppHandle, state: State<'_, BackupSystem>) {
    // アプリ起動時の初期化処理
    // 1. 設定情報の現在値をフロントエンドへ通知
    let settings = state.get_settings();
    let _ = app.emit("settings-state", settings);

    // 2. バックアップの状態（フォルダ一覧や履歴）をスキャンしてフロントエンドへ通知
    // 実行ファイルのディレクトリにある「Backups」フォルダをルートとして使用
    let exe_path = std::env::current_exe().unwrap_or_default();
    let backups_root = exe_path.parent().unwrap().join("Backups");
    BackupSystem::emit_state(&app, &backups_root);
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    repo_path: String,
    max_generations: Option<usize>,
    theme: Option<String>,
) {
    // 設定の保存処理
    let mk = SettingsManager::new();
    let mut current = mk.load();

    // パスを更新
    current.repo_save_path = repo_path;

    // 世代制限の上限を確認して反映 (1〜100)
    if let Some(gen) = max_generations {
        current.max_generations = gen.clamp(1, 100);
    }

    // テーマを反映
    if let Some(t) = theme {
        current.theme = t;
    }

    // ファイル（settings.ini）へ保存
    mk.save(&current);

    // バックアップ監視システム側の設定をリアルタイムで同期
    let backup_system: tauri::State<BackupSystem> = app.state();
    backup_system.update_settings(current.clone());

    // フロントエンドへ最新の設定状態を通知
    let _ = app.emit("settings-state", current);

    // 監視対象パスが変更された可能性があるため、バックアップ一覧を再取得
    let exe_path = std::env::current_exe().unwrap_or_default();
    let backups_root = exe_path.parent().unwrap().join("Backups");
    BackupSystem::emit_state(&app, &backups_root);
}

#[tauri::command]
fn manual_backup(state: State<'_, BackupSystem>, subfolder_name: String) {
    // ユーザーがUIから「今すぐバックアップ」ボタンを押した際に呼ばれる
    // 引数 subfolder_name: バックアップ対象のサブフォルダ名（例: "SaveData1"）
    state.trigger_backup(&subfolder_name);
}

#[tauri::command]
fn restore_backup(state: State<'_, BackupSystem>, subfolder_name: String, timestamp: String) {
    // ユーザーがUIから特定のバックアップを選択してリストアする際に呼ばれる
    // 引数 subfolder_name: リストア対象のサブフォルダ名
    // 引数 timestamp: リストアに使用するバックアップのタイムスタンプ（フォルダ名）
    let settings = state.get_settings();
    let exe_path = std::env::current_exe().unwrap_or_default();
    let backups_root = exe_path.parent().unwrap().join("Backups");

    state.restore_backup(&settings, &backups_root, &subfolder_name, &timestamp);
}

#[tauri::command]
fn delete_backup(app: tauri::AppHandle, subfolder_name: String, timestamp: String) {
    let exe_path = std::env::current_exe().unwrap_or_default();
    let backups_root = exe_path.parent().unwrap().join("Backups");

    BackupSystem::delete_backup(&app, &backups_root, &subfolder_name, &timestamp);
    BackupSystem::emit_state(&app, &backups_root);
}

#[tauri::command]
fn delete_subfolder(app: tauri::AppHandle, subfolder_name: String) {
    let exe_path = std::env::current_exe().unwrap_or_default();
    let backups_root = exe_path.parent().unwrap().join("Backups");

    BackupSystem::delete_subfolder(&app, &backups_root, &subfolder_name);
    BackupSystem::emit_state(&app, &backups_root);
}

#[tauri::command]
fn save_memo(app: tauri::AppHandle, subfolder_name: String, memo_content: String) {
    // サブフォルダ（カテゴリ）ごとのメモを meta.json に保存する
    let exe_path = std::env::current_exe().unwrap_or_default();
    let backups_root = exe_path.parent().unwrap().join("Backups");

    BackupSystem::save_memo(&backups_root, &subfolder_name, &memo_content);
    // 保存後、UIを即座に更新するために状態を再送
    BackupSystem::emit_state(&app, &backups_root);
}

#[cfg(debug_assertions)]
#[tauri::command]
fn test_notification(app: tauri::AppHandle) {
    println!("[DEBUG COMMAND] test_notification called");
    // 開発時のデバッグ用コマンド。実際のバックアップ生成時と同じ流れで通知を表示します。
    BackupSystem::send_notification(&app, "テスト通知", "これはデバッグ用のテスト通知です。");
}

#[tauri::command]
fn open_path_in_explorer(app: tauri::AppHandle, path: String) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_path(path, None::<&str>);
}

/// バックアップルートフォルダをエクスプローラで開きます。
#[tauri::command]
fn open_backups_folder(app: tauri::AppHandle) {
    use tauri_plugin_opener::OpenerExt;
    let exe_path = std::env::current_exe().unwrap_or_default();
    let backups_root = exe_path.parent().unwrap().join("Backups");
    let _ = app
        .opener()
        .open_path(backups_root.to_string_lossy().to_string(), None::<&str>);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 既に起動している場合、メインウィンドウにフォーカス
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // アプリのバージョン情報を取得してウィンドウタイトルに反映
            let package_info = app.package_info();
            let version = &package_info.version;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&format!("RepoSaver v{}", version));
            }

            // 設定の初期化
            let mk = SettingsManager::new();
            let settings = mk.load();

            // BackupSystem の初期化
            let backup_system = BackupSystem::new(app.handle().clone(), settings.clone());
            backup_system.start_watcher();

            // 状態管理への登録
            app.manage(backup_system);

            // 通知専用のウィンドウ（オーバーレイ）をバックグラウンドで作成
            // システム通知が他ウィンドウに隠れないよう、常に最前面(transparent + always_on_top)に配置
            {
                use tauri::WebviewUrl;

                // プライマリモニターの情報を取得して、通知を表示する「右下」の座標を算出
                let monitors = app.available_monitors().unwrap_or_default();
                let monitor = monitors.first().cloned().unwrap_or_else(|| {
                    panic!("No monitors available");
                });

                let screen_size = monitor.size();
                let scale_factor = monitor.scale_factor();

                // 通知ウィンドウの論理サイズ
                let win_width = 360.0;
                let win_height = 90.0;
                let padding = 72.0;

                // 物理ピクセルに変換して正確な座標を特定
                let physical_width = (win_width * scale_factor) as i32;
                let physical_height = (win_height * scale_factor) as i32;
                let physical_padding = (padding * scale_factor) as i32;

                // 画面の幅/高さからウィンドウサイズと余白を引いて右下位置を算出
                let x = screen_size.width as i32 - physical_width - physical_padding;
                let y = screen_size.height as i32 - physical_height - physical_padding;

                // WebviewWindow の作成
                let _ = tauri::WebviewWindowBuilder::new(
                    app,
                    "notification",
                    WebviewUrl::App("index.html#/notification".into()),
                )
                .title("通知")
                .inner_size(win_width, win_height)
                .position(x as f64, y as f64)
                .decorations(false) // 枠なし
                .transparent(true) // 透過有効
                .always_on_top(true) // 常に最前面
                .resizable(false)
                .skip_taskbar(true) // タスクバーには表示しない
                .visible(false) // 初期状態は非表示（イベント受信時に表示制御）
                .build()?;
            }

            // システムトレイ
            let quit_i = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let settings_i =
                MenuItem::with_id(app, "settings", "設定画面を開く", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        // アプリを完全に終了
                        app.exit(0);
                    }
                    "settings" => {
                        // 設定画面を前面に表示
                        if let Some(win) = app.get_webview_window("main") {
                            win.show().unwrap();
                            win.set_focus().unwrap();
                            // フロントエンドへ設定画面への遷移を指示
                            let _ = win.emit("navigate-settings", ());
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        // 左クリックでウィンドウの表示/非表示をトグル
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                win.hide().unwrap();
                            } else {
                                win.show().unwrap();
                                win.set_focus().unwrap();
                                // 表示時は常にログイン後のホーム（一覧）画面へ遷移させる
                                app.emit("navigate-home", ()).unwrap();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // ウィンドウの「×」ボタンが押された際、アプリを終了せずタスクトレイへ常駐させる
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            initialize_app,
            save_settings,
            manual_backup,
            restore_backup,
            delete_backup,
            delete_subfolder,
            save_memo,
            open_path_in_explorer,
            open_backups_folder,
            #[cfg(debug_assertions)]
            test_notification
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
