<script setup>
import { ref, watch } from 'vue'
import { useMainStore } from '../stores/mainStore'

const store = useMainStore()

const localPath = ref('')
const maxGenerations = ref(10)
const theme = ref('system')
let pathTimer = null
let generationsTimer = null

watch(() => store.settings.repo_save_path, (newVal) => {
  localPath.value = newVal
}, { immediate: true })

watch(() => store.settings.max_generations, (newVal) => {
  maxGenerations.value = newVal || 10
}, { immediate: true })

watch(() => store.settings.theme, (newVal) => {
  theme.value = newVal || 'system'
}, { immediate: true })

// パス入力の変更を検知して自動保存（入力停止から500ms後に実行）
const onPathInput = () => {
  if (pathTimer) clearTimeout(pathTimer)
  pathTimer = setTimeout(() => {
    store.saveSettings(localPath.value, maxGenerations.value, theme.value)
  }, 500)
}

// 保持世代数の変更を検知して自動保存。バリデーションも含める。
const onMaxGenerationsInput = () => {
  // 入力範囲を制限 (バックエンド側でも制限しているが、UIの反応性を高めるため)
  if (maxGenerations.value < 1) maxGenerations.value = 1
  if (maxGenerations.value > 100) maxGenerations.value = 100
  
  if (generationsTimer) clearTimeout(generationsTimer)
  generationsTimer = setTimeout(() => {
    store.saveSettings(localPath.value, maxGenerations.value, theme.value)
  }, 500)
}

const onThemeChange = (newTheme) => {
  store.saveSettings(localPath.value, maxGenerations.value, newTheme)
}
</script>

<template>
  <v-container fluid class="pa-4">
    <v-row>
      <v-col cols="12">
        <div class="text-h5 mb-4 font-weight-bold">設定</div>
        
        <v-card variant="elevated" elevation="1" class="pa-4 rounded-lg mb-4">
           <!-- 監視対象のフォルダパス。@input イベントでデバウンスを介して自動保存されます -->
           <v-text-field
             v-model="localPath"
             label="R.E.P.O. セーブデータパス"
             placeholder="C:\Users\...\Repo\saves"
             @input="onPathInput"
             hint="セーブデータの変更を監視する対象フォルダです。"
             persistent-hint
             prepend-inner-icon="mdi-folder-open"
             variant="outlined"
             color="primary"
           ></v-text-field>
        </v-card>

        <v-card variant="elevated" elevation="1" class="pa-4 rounded-lg mb-4">
          <div class="text-subtitle-1 mb-3 font-weight-bold">バックアップ設定</div>
          <!-- バックアップの保持数。変更は 500ms のディレイ後に自動的に保存されます -->
          <v-text-field
            v-model.number="maxGenerations"
            label="保持件数"
            type="number"
            min="1"
            max="100"
            @input="onMaxGenerationsInput"
            hint="各フォルダの最大バックアップ保持数（1〜100件）"
            persistent-hint
            prepend-inner-icon="mdi-counter"
            variant="outlined"
            color="primary"
          ></v-text-field>
        </v-card>

        <v-card variant="elevated" elevation="1" class="pa-4 rounded-lg mb-4">
          <div class="text-subtitle-1 mb-3 font-weight-bold">テーマ設定</div>
          <v-radio-group v-model="theme" @update:model-value="onThemeChange">
            <v-radio label="ダークモード" value="dark"></v-radio>
            <v-radio label="ライトモード" value="light"></v-radio>
            <v-radio label="システム設定に従う" value="system"></v-radio>
          </v-radio-group>
        </v-card>

      </v-col>
    </v-row>
  </v-container>
</template>

<style scoped>
/* Vuetifyがスタイルを処理します */
</style>
