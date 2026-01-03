<script setup>
import { useMainStore } from '../stores/mainStore'
import { storeToRefs } from 'pinia'
import { ref, watch, computed } from 'vue'
import dayjs from 'dayjs'

const store = useMainStore()
const { items, isConnected } = storeToRefs(store)

// 各カテゴリ（サブフォルダ）が展開されているかどうかの状態保持
const expanded = ref({}) // { 'FolderName': true/false }

const toggle = (name) => {
  expanded.value[name] = !expanded.value[name]
}

// メモ（自由記述）の編集用ローカルステート
const memos = ref({})
// 入力停止を待ってから保存するためのタイマー
let memoTimer = null

// ストアのデータが更新されたら、ローカルのメモ入力欄に反映
// ただし、現在入力中の項目（フォーカスがある項目）は上書きしないように制御
watch(items, (newItems) => {
  newItems.forEach(item => {
    if (memos.value[item.name] === undefined || (memos.value[item.name] !== item.memo && !document.activeElement?.className?.includes('memo-input'))) {
      memos.value[item.name] = item.memo
    }
  })
}, { deep: true, immediate: true })

const onMemoInput = (folderName) => {
  if (memoTimer) clearTimeout(memoTimer)
  memoTimer = setTimeout(() => {
    store.saveMemo(folderName, memos.value[folderName])
  }, 1000)
}

const formatDate = (tsStr) => {
  // バックエンドからのタイムスタンプ（YYYYMMDD_HHmmss）を読みやすい形式に変換
  return dayjs(tsStr, "YYYYMMDD_HHmmss").format("YYYY-MM-DD HH:mm:ss")
}

const manualBackup = (folderName) => {
  store.triggerBackup(folderName)
}

const restore = async (folderName, timestamp) => {
  await store.restore(folderName, timestamp)
}

// 削除ダイアログのロジック
const dialogDelete = ref(false)
const deleteTarget = ref(null) // { type: 'subfolder'|'backup', name: '', timestamp: '' }

const deleteDescription = computed(() => {
    if (!deleteTarget.value) return ''
    if (deleteTarget.value.type === 'subfolder') {
        return `フォルダ「${deleteTarget.value.name}」のすべてのバックアップを削除しますか？この操作は取り消せません。`
    } else {
        return `${formatDate(deleteTarget.value.timestamp)} のバックアップを削除しますか？`
    }
})

// 特定のバックアップ一個の個別の削除
const deleteBk = (folderName, timestamp) => {
    deleteTarget.value = { type: 'backup', name: folderName, timestamp }
    dialogDelete.value = true
}

// サブフォルダ内のすべてのバックアップを削除（カテゴリ全体の削除相当）
const deleteFolder = (folderName) => {
    deleteTarget.value = { type: 'subfolder', name: folderName }
    dialogDelete.value = true
}

const confirmDelete = async () => {
    dialogDelete.value = false
    if (!deleteTarget.value) return
    
    if (deleteTarget.value.type === 'subfolder') {
        await store.deleteSubfolder(deleteTarget.value.name)
    } else {
        await store.deleteBackup(deleteTarget.value.name, deleteTarget.value.timestamp)
    }
    deleteTarget.value = null
}
</script>

<template>
  <v-container fluid class="fill-height align-start pa-4">
    <!-- ローディング中 -->
    <v-row v-if="!isConnected" justify="center" align="center" class="fill-height">
        <v-col cols="auto" class="text-center">
            <v-progress-circular indeterminate color="primary" size="64"></v-progress-circular>
            <div class="mt-4 text-medium-emphasis">バックエンドに接続中...</div>
        </v-col>
    </v-row>
    
    <v-row v-else-if="items.length === 0" justify="center" align="center" class="fill-height">
         <v-col cols="auto" class="text-center text-medium-emphasis">
            <v-icon size="64" class="mb-4">mdi-folder-search-outline</v-icon>
            <div>リポジトリパス内にサブフォルダが見つかりませんでした。</div>
         </v-col>
    </v-row>

    <v-row v-else>
      <v-col cols="12" class="py-1" v-for="item in items" :key="item.name">
        <v-card variant="elevated" elevation="2" class="rounded-lg">
            <!-- カード上部: アイコン、タイトル、ボタン（展開/折りたたみ、フォルダアイコン） -->
            <v-card-item class="pl-2">
                <template v-slot:prepend>
                    <v-list-item-action start>
                     <!-- サブフォルダの詳細（バックアップ履歴）を表示/非表示 -->
                     <v-btn icon variant="text" size="small" @click="toggle(item.name)">
                        <v-icon>{{ expanded[item.name] ? 'mdi-chevron-down' : 'mdi-chevron-right' }}</v-icon>
                     </v-btn>
                    </v-list-item-action>
                    <v-icon color="primary" class="mr-1" size="medium">mdi-folder</v-icon>
                </template>
                
                <v-card-title class="d-flex flex-column align-start text-body-1 py-1">
                    <div class="font-weight-bold">{{ item.name }}</div>
                    <div class="text-caption text-medium-emphasis mt-n1">
                        <v-icon size="x-small" class="mr-1">mdi-clock-outline</v-icon>
                        <span v-if="item.backups && item.backups.length > 0">
                            最新: {{ formatDate(item.backups[0].timestamp) }}
                        </span>
                        <span v-else>バックアップ履歴なし</span>
                    </div>
                </v-card-title>
                
                <!-- メモとアクション -->
                <template v-slot:append>
                    <div class="d-flex align-center">
                        <v-text-field 
                           v-model="memos[item.name]"
                           @input="onMemoInput(item.name)"
                           density="compact"
                           variant="underlined"
                           hide-details
                           placeholder="Memo..."
                           prepend-inner-icon="mdi-note-text-outline"
                           class="mr-6 memo-input"
                           style="min-width: 200px; max-width: 300px;"
                           :disabled="item.backups.length === 0"
                        ></v-text-field>

                        <v-tooltip text="最新をリストア" location="top">
                            <template v-slot:activator="{ props }">
                                <v-btn 
                                    v-bind="props" 
                                    icon 
                                    variant="text" 
                                    color="success" 
                                    @click="restore(item.name, item.backups[0].timestamp)"
                                    :disabled="item.backups.length === 0"
                                >
                                    <v-icon>mdi-restore</v-icon>
                                </v-btn>
                            </template>
                        </v-tooltip>

                        <v-tooltip text="今すぐバックアップ" location="top">
                            <template v-slot:activator="{ props }">
                                <v-btn v-bind="props" icon variant="text" color="primary" @click="manualBackup(item.name)" :disabled="!item.source_exists">
                                    <v-icon>mdi-floppy</v-icon>
                                </v-btn>
                            </template>
                        </v-tooltip>
                        
                        <v-tooltip text="全てのバックアップを削除" location="top">
                            <template v-slot:activator="{ props }">
                                <v-btn v-bind="props" icon variant="text" color="error" @click="deleteFolder(item.name)" :disabled="item.backups.length === 0">
                                    <v-icon>mdi-delete-sweep</v-icon>
                                </v-btn>
                            </template>
                        </v-tooltip>
                    </div>
                </template>
            </v-card-item>
            
            <!-- 展開されたコンテンツ (バックアップ一覧) -->
            <v-expand-transition>
                <div v-if="expanded[item.name]">
                     <v-divider></v-divider>
                     <div v-if="item.backups.length === 0" class="pa-4 text-center text-medium-emphasis text-caption">
                        バックアップがありません。
                     </div>
                     <v-list v-else density="compact" bg-color="transparent" class="py-0">
                        <v-list-item v-for="bk in item.backups" :key="bk.timestamp" lines="one">
                             <template v-slot:prepend>
                                 <v-icon size="small" color="grey" class="mr-2">mdi-clock-outline</v-icon>
                             </template>
                             <v-list-item-title class="font-mono text-body-2">
                                 {{ formatDate(bk.timestamp) }}
                             </v-list-item-title>
                             
                             <template v-slot:append>
                                 <div class="d-flex gap-2">
                                     <v-btn icon size="x-small" variant="text" color="success" @click="restore(item.name, bk.timestamp)" title="リストア">
                                         <v-icon>mdi-restore</v-icon>
                                     </v-btn>
                                     <v-btn icon size="x-small" variant="text" color="error" @click="deleteBk(item.name, bk.timestamp)" title="削除">
                                         <v-icon>mdi-delete</v-icon>
                                     </v-btn>
                                 </div>
                             </template>
                        </v-list-item>
                     </v-list>
                </div>
            </v-expand-transition>
        </v-card>
      </v-col>
    </v-row>
    
    <!-- 削除確認ダイアログ -->
    <v-dialog v-model="dialogDelete" max-width="450">
        <v-card prepend-icon="mdi-alert" title="削除の確認">
            <v-card-text>
                {{ deleteDescription }}
            </v-card-text>
            <v-card-actions>
                <v-spacer></v-spacer>
                <v-btn color="grey-lighten-1" variant="text" @click="dialogDelete = false">キャンセル</v-btn>
                <v-btn color="error" variant="elevated" @click="confirmDelete">削除</v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>

  </v-container>
</template>

<style scoped>
/* Vuetifyのユーティリティで対応できない細かい調整のみここに記述 */
.font-mono {
    /* タイムスタンプ（数字）の列を揃えやすくするために等幅フォントを適用 */
    font-family: monospace;
}
</style>
