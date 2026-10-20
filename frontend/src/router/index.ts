import { createRouter, createWebHistory } from 'vue-router'
import VideoCallView from '../views/VideoCallView.vue'
import HomeView from '@/views/HomeView.vue'
import SigninView from '@/views/SigninView.vue'
import { useAuthStore } from '@/stores/auth'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      component: HomeView,
      meta: {
        title: 'Home',
        requiresAuth: true,
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
        requiresAuth: true,
      },
    },
  ],
})

router.beforeEach(async (to) => {
  const auth = useAuthStore()

  if (to.meta.requiresAuth && !auth.authenticated) {
    // one-shot restore from the httpOnly refresh cookie
    const restored = await auth.refreshAccessToken()
    if (!restored) {
      return { path: '/signin' }
    }
  }

  const titleFromParams = to.params?.pageTitle
  const title = titleFromParams
    ? String(titleFromParams)
    : String(to.meta?.title || 'Default Title')

  document.title = `${title} | VidCall`

  return true
})

export default router
