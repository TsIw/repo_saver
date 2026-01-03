import { createRouter, createWebHashHistory } from 'vue-router'
import SaveList from '../components/SaveList.vue'
import Settings from '../components/Settings.vue'
import NotificationOverlay from '../components/NotificationOverlay.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: SaveList
    },
    {
      path: '/settings',
      name: 'settings',
      component: Settings
    },
    {
      path: '/notification',
      name: 'notification',
      component: NotificationOverlay
    }
  ]
})

export default router
