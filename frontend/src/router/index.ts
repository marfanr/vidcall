import { createRouter, createWebHistory } from 'vue-router'
import VideoCallView from '../views/VideoCallView.vue'
import HomeView from '@/views/HomeView.vue'
import SigninView from '@/views/SigninView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: HomeView,
    },
    {
      path: '/signin',
      component: SigninView,
    },
    {
      path: '/meet/:id',
      name: 'video-call',
      component: VideoCallView,
    },
  ],
})

export default router
