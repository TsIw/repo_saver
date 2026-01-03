# API インターフェース定義

Backend (Rust) と Frontend (Vue.js) の間の通信仕様について記述します。

## 1. Tauri Commands (Frontend -> Backend)
フロントエンドから `invoke` 関数を使用して呼び出すことができるコマンドです。

| コマンド名 | 引数 | 説明 |
| :-- | :-- | :-- |
| `initialize_app` | - | アプリ起動時に初期データ（設定・状態）のブロードキャストを要求します。 |
| `save_settings` | `repo_path`, `max_generations`, `theme` | 設定情報を保存し、バックエンドの監視システムを更新します。 |
| `manual_backup` | `subfolder_name` | 指定したサブフォルダのバックアップを即座に実行します。 |
| `restore_backup` | `subfolder_name`, `timestamp` | 指定した時点のバックアップをごみ箱を避けつつ復元します。 |
| `delete_backup` | `subfolder_name`, `timestamp` | 特定のバックアップフォルダを削除します。 |
| `delete_subfolder`| `subfolder_name` | 特定のバックアップカテゴリ（フォルダ）全体を削除します。 |
| `save_memo` | `subfolder_name`, `memo_content` | 各カテゴリの `meta.json` にメモを保存します。 |
| `open_path_in_explorer` | `path` | 指定したパスをエクスプローラで開きます。 |
| `open_backups_folder` | - | バックアップルートフォルダをエクスプローラで開きます。 |

## 2. Tauri Events (Backend -> Frontend)
バックエンドから特定のタイミング、または全局的に発行されるイベントです。

| イベント名 | ペイロード | 発行タイミング |
| :-- | :-- | :-- |
| `settings-state` | `Settings` オブジェクト | 設定が変更された、または初期化されたとき。 |
| `backups-state` | `Vec<FolderState>` | バックアップ一覧に変化があったとき。 |
| `notification` | `Message`, `Type` | バックアップ完了、エラー発生、リストア完了などの通知時。 |
| `navigate-home` | - | トレイアイコンクリック時など、ホームへの遷移を促すとき。 |
| `navigate-settings`| - | トレイメニューで「設定」が選ばれたとき。 |

## 3. 型定義 (TypeScriptライクな定義)
フロントエンドで管理される主なデータ構造：

```typescript
interface Settings {
  repo_save_path: string;
  max_generations: number;
  theme: 'dark' | 'light' | 'system';
}

interface BackupItem {
  timestamp: string;
  is_auto: boolean;
}

interface FolderState {
  name: string;
  backups: BackupItem[];
  latest_backup?: string;
  memo?: string;
}
```
