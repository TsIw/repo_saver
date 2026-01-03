<script setup>
import { computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useMainStore } from './stores/mainStore'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useTheme } from 'vuetify'

const router = useRouter()
const store = useMainStore()
const vuetifyTheme = useTheme()

let unlistenHome = null
let unlistenSettings = null

// グローバルテーマを適用する内部関数
// Vuetifyのテーマエンジンを使用して、ライト/ダーク/システム設定をアプリ全体に反映します
const applyTheme = (themeValue) => {
  if (!themeValue) return
  const target = themeValue === 'system' 
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : themeValue
  
  // [Vuetify UPGRADE] 警告に対応: global.name.value ではなく name.value を試行
  // または警告の指示通り change() メソッドがあるか確認
  if (typeof vuetifyTheme.change === 'function') {
    vuetifyTheme.change(target)
  } else {
    vuetifyTheme.global.name.value = target
  }
}

// ストア（永続化された設定）のテーマが変更されたら即座に反映
// immediate: true により起動直後にもこの処理が走り、正しいテーマで開始されます
watch(() => store.settings.theme, (newTheme) => {
  applyTheme(newTheme)
}, { immediate: true })

onMounted(async () => {
  // バックエンドからの初期データ（設定やバックアップ一覧）をリクエスト
  store.init()
  
  // システムトレイやバックエンドからの画面遷移要求イベントを登録
  unlistenHome = await listen('navigate-home', () => {
    navigate('home')
  })
  unlistenSettings = await listen('navigate-settings', () => {
    navigate('settings')
  })
})

onUnmounted(() => {
  if (unlistenHome) unlistenHome()
  if (unlistenSettings) unlistenSettings()
})

const activeTab = computed(() => {
  return route.path === '/settings' ? 'settings' : 'home'
})

const isNotificationWindow = computed(() => {
  // router の初期化を待たずに判定できるよう、URLを直接確認
  return window.location.hash.includes('notification') || window.location.pathname.includes('notification')
})

const navigate = (tab) => {
  if (tab === 'home') router.push('/')
  else router.push('/settings')
}

const openRepoFolder = async () => {
  const path = store.settings.repo_save_path
  if (path) {
    try {
      await invoke('open_path_in_explorer', { path })
    } catch (e) {
      console.error("フォルダを開けませんでした:", e)
    }
  }
}

const openBackupsFolder = async () => {
  try {
    await invoke('open_backups_folder')
  } catch (e) {
    console.error("バックアップフォルダを開けませんでした:", e)
  }
}
</script>

<template>
  <v-app v-if="!isNotificationWindow">
    <v-navigation-drawer
      permanent
      rail
      color="surface"
    >
      <v-list>
        <v-tooltip text="バックアップリスト" location="right">
          <template v-slot:activator="{ props }">
            <v-list-item
              v-bind="props"
              prepend-icon="mdi-backup-restore"
              title="バックアップリスト"
              value="backups"
              to="/"
            ></v-list-item>
          </template>
        </v-tooltip>

        <v-tooltip text="設定" location="right">
          <template v-slot:activator="{ props }">
            <v-list-item
              v-bind="props"
              prepend-icon="mdi-cog"
              title="設定"
              value="settings"
              to="/settings"
            ></v-list-item>
          </template>
        </v-tooltip>
      </v-list>

      <template v-slot:append>
        <div class="pa-2 d-flex flex-column align-center gap-2">
            <v-tooltip text="監視フォルダを開く" location="right">
                <template v-slot:activator="{ props }">
                    <v-btn
                        v-bind="props"
                        icon="mdi-folder-open"
                        variant="text"
                        @click="openRepoFolder"
                    ></v-btn>
                </template>
            </v-tooltip>

            <v-tooltip text="バックアップフォルダを開く" location="right">
                <template v-slot:activator="{ props }">
                    <v-btn
                        v-bind="props"
                        icon="mdi-folder-clock"
                        variant="text"
                        @click="openBackupsFolder"
                    ></v-btn>
                </template>
            </v-tooltip>
        </div>
      </template>
    </v-navigation-drawer>

    <v-main>
      <router-view v-slot="{ Component }">
          <component :is="Component" />
      </router-view>
    </v-main>
  </v-app>
  <router-view v-else />
</template>

<style scoped>
/* Vuetifyがほとんどのスタイルを処理します。
   必要に応じてオーバーライドを行いますが、当面はデフォルトに任せます。
*/
</style>
