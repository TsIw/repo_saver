import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import dayjs from 'dayjs'

export const useMainStore = defineStore('main', {
  state: () => ({
    settings: {
      repo_save_path: '',
      theme: 'system',
      max_generations: 10
    },
    items: [], // [{ name, memo, backups: [{timestamp, timestamp_raw}], source_exists }]
    isConnected: false
  }),

  actions: {
    async init() {
      // バックグラウンド側（Rust）からの設定変更や状態の同期イベントを常時待受
      await listen('settings-state', (event) => {
        this.settings = event.payload
      })

      await listen('backups-state', (event) => {
        this.items = event.payload
      })

      // システムトレイ側からの画面遷移要求をリッスン
      await listen('navigate-settings', () => {
        // App.vue 側などで activeTab を切り替えるためのトリガー
      })

      // 初期状態をリクエスト
      try {
        await invoke('initialize_app')
        this.isConnected = true
      } catch (e) {
        console.error("初期化に失敗しました", e)
      }
    },

    async saveSettings(newPath, maxGenerations, theme) {
      // ユーザー設定（パス、保持世代、テーマ）をバックグラウンドへ保存
      // 引数が未指定の場合は現在のストアの値をデフォルトとして採用
      await invoke('save_settings', {
        repoPath: newPath,
        maxGenerations: maxGenerations || this.settings.max_generations || 10,
        theme: theme || this.settings.theme || 'system'
      })
    },

    async triggerBackup(subfolderName) {
      // 特定のサブフォルダに対して即座にバックアップを実行させる
      await invoke('manual_backup', { subfolderName })
    },

    async restore(subfolderName, timestamp) {
      // 指定したタイムスタンプのバックアップを元の場所へ書き戻す
      await invoke('restore_backup', { subfolderName, timestamp })
    },

    async deleteBackup(subfolderName, timestamp) {
      await invoke('delete_backup', { subfolderName, timestamp })
    },

    async deleteSubfolder(subfolderName) {
      await invoke('delete_subfolder', { subfolderName })
    },

    async saveMemo(subfolderName, content) {
      // サブフォルダに対するメモを保存
      // バックエンド側で meta.json への書き出しが行われる
      await invoke('save_memo', { subfolderName, memoContent: content })
    }
  }
})
