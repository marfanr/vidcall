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
      meta: {
        title: 'Sign-in',
      },
    },
    {
      path: '/meet/:id',
      name: 'video-call',
      component: VideoCallView,
    },
  ],
})

router.beforeEach((to, from, next) => {
  const titleFromParams = to.params?.pageTitle

  if (titleFromParams) {
    document.title = `${titleFromParams} - ${document.title}`
  } else {
    document.title = String(to.meta?.title || 'Default Title')
  }

  next()
})

export default router
