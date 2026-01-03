<script setup>
import { computed, onMounted, onUnmounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useMainStore } from './stores/mainStore'
import { listen } from '@tauri-apps/api/event'
import { useTheme } from 'vuetify'

const router = useRouter()
const route = useRoute()
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
  return route.path === '/notification'
})

const navigate = (tab) => {
  if (tab === 'home') router.push('/')
  else router.push('/settings')
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
        <v-list-item
          prepend-icon="mdi-backup-restore"
          title="バックアップリスト"
          value="backups"
          to="/"
        ></v-list-item>
        <v-list-item
          prepend-icon="mdi-cog"
          title="設定"
          value="settings"
          to="/settings"
        ></v-list-item>
      </v-list>
    </v-navigation-drawer>

    <v-main>
      <router-view v-slot="{ Component }">
        <keep-alive>
          <component :is="Component" />
        </keep-alive>
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
