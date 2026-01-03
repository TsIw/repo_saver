<script setup>
import { ref, onMounted, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { currentMonitor } from '@tauri-apps/api/window'

const message = ref('')
const title = ref('')
const type = ref('info')
const visible = ref(false)
let timer = null

const icons = {
  backup: 'mdi-floppy',
  restore: 'mdi-restore',
  delete: 'mdi-delete',
  success: 'mdi-check-circle'
}

// 通知の種類に応じたアイコンを選択
const currentIcon = computed(() => {
    if (title.value.includes('バックアップ')) return icons.backup
    if (title.value.includes('リストア')) return icons.restore
    if (title.value.includes('削除')) return icons.delete
    return icons.success
})

// 通知の種類に応じたVuetifyのカラーバリエーションを選択
const typeColor = computed(() => {
    if (title.value.includes('バックアップ')) return 'primary'
    if (title.value.includes('リストア')) return 'success'
    if (title.value.includes('削除')) return 'error'
    return 'success'
})

onMounted(async () => {
  console.log('[NOTIFICATION OVERLAY] Component mounted')
  document.body.style.backgroundColor = 'transparent'
  document.documentElement.style.backgroundColor = 'transparent'

  // 位置はRust側のウィンドウ作成時に処理されるため、
  // ここでの再配置は不要です

  // Rustからの 'show-notification' イベントをリッスン
  console.log('[NOTIFICATION OVERLAY] show-notification イベントの待受を開始します')
  await listen('show-notification', async (event) => {
    console.log('[NOTIFICATION OVERLAY] 通知イベントを受信:', event)
    const payload = event.payload
    console.log('[NOTIFICATION OVERLAY] Payload:', payload)
    
    title.value = payload.title
    message.value = payload.body
    type.value = payload.type || 'info'
    
    // UI上のカードを表示状態にする
    visible.value = true
    console.log('[NOTIFICATION OVERLAY] 通知を表示状態に設定しました')
    
    // ウィンドウ自体も不可視状態(visible: false)で作成されているため、OSに対して「表示」を要求
    const win = getCurrentWindow()
    console.log('[NOTIFICATION OVERLAY] ウィンドウを表示します')
    await win.show()
    console.log('[NOTIFICATION OVERLAY] ウィンドウが表示されました')

    if (timer) clearTimeout(timer)
    timer = setTimeout(async () => {
      console.log('[NOTIFICATION OVERLAY] 通知を自動非表示にします')
      visible.value = false
      setTimeout(async () => {
          console.log('[NOTIFICATION OVERLAY] ウィンドウを隠します')
          await win.hide()
      }, 300) 
    }, 3000)
  })
  console.log('[NOTIFICATION OVERLAY] Event listener registered successfully')
})
</script>

<template>
  <v-app style="background: transparent;" class="notification-overlay-root">
    <v-fade-transition>
        <v-card 
           v-if="visible"
           class="notification-card"
           :color="typeColor"
           elevation="8"
           theme="dark"
           rounded="0"
        >
            <div class="d-flex align-center pa-4" style="height: 100%;">
                <v-icon size="32" class="mr-4">{{ currentIcon }}</v-icon>
                <div class="flex-grow-1">
                    <div class="text-subtitle-2 font-weight-bold">{{ title }}</div>
                    <div class="text-caption">{{ message }}</div>
                </div>
            </div>
        </v-card>
    </v-fade-transition>
  </v-app>
</template>

<style>
/* 
   通知ウィンドウ専用のルートクラスが存在する場合のみ、html/body を調整します。
   これにより、メインウィンドウなどの他ウィンドウへの干渉を防ぎます。
*/
html:has(.notification-overlay-root),
body:has(.notification-overlay-root) {
  margin: 0 !important;
  padding: 0 !important;
  width: 100% !important;
  height: 100% !important;
  overflow: hidden !important;
  background: transparent !important;
}
</style>

<style scoped>
#app {
  margin: 0;
  padding: 0;
  background: transparent !important;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.v-application {
  background: transparent !important;
  width: 100%;
  height: 100%;
}

.v-application__wrap {
  min-height: 100% !important;
  width: 100% !important;
}

.notification-card {
   box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
   width: 100%;
   height: 100%;
   margin: 0 !important;
   border-radius: 0 !important;
}

.v-card {
  margin: 0 !important;
  border-radius: 0 !important;
}
</style>
