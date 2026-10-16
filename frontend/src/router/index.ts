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
      meta: {
        title: 'Home',
      },
    },
    {
      path: '/signin',
      component: SigninView,
      meta: {
        title: 'Sign-in',
      },
    },
    {
      path: '/meet/:id',
      name: 'video-call',
      component: VideoCallView,
      meta: {
        title: 'Video Call',
      },
    },
  ],
})

router.beforeEach((to) => {
  const titleFromParams = to.params?.pageTitle

  const title = titleFromParams
    ? String(titleFromParams)
    : String(to.meta?.title || 'Default Title')

  document.title = `${title} | VidCall`

  return true
})

export default router
