import { createRouter, createWebHistory } from 'vue-router'
import VideoCallView from '../views/VideoCallView.vue'
import HomeView from '@/views/HomeView.vue'
import SigninView from '@/views/SigninView.vue'
import { api } from '@/lib/axios.ts'
// import { useAuthStore } from '@/stores/auth'
// import { api } from '@/lib/axios.ts'
// import { info } from 'console'

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

router.beforeEach((to) => {
  // const auth = useAuthStore()

  // if (to.meta.requiresAuth && !auth.authenticated) {
  //   return { path: '/signin' }
  // }

  // const cookies = useCookies(['locale']).getAll()
  // console.log(cookies)

  api.post('/auth/refresh').then((r) => console.log(r))

  const titleFromParams = to.params?.pageTitle
  const title = titleFromParams
    ? String(titleFromParams)
    : String(to.meta?.title || 'Default Title')

  document.title = `${title} | VidCall`

  return true
})

export default router
