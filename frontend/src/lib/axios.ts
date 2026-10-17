import axios from 'axios'
import router from '@/router'
import { useAuthStore } from '@/stores/auth'

export const api = axios.create({
  baseURL: 'http://localhost:8080/v1',
  timeout: 10_000,
  headers: {
    Accept: 'application/json',
  },
  decompress: true,
  fetchOptions: {},
  withCredentials: true,
})

api.interceptors.request.use((config) => {
  const auth = useAuthStore()
  if (auth.accessToken) {
    config.headers.Authorization = `Bearer ${auth.accessToken}`
  }
  return config
})

let isRefreshing = false
let refreshSubscribers: ((token: string) => void)[] = []

function subscribeTokenRefresh(cb: (token: string) => void) {
  refreshSubscribers.push(cb)
}

function onRefreshComplete(newToken: string) {
  refreshSubscribers.forEach((cb) => cb(newToken))
  refreshSubscribers = []
}

api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config
    const auth = useAuthStore()

    if (error.response?.status === 401 && !originalRequest._retry) {
      if (isRefreshing) {
        return new Promise((resolve) => {
          subscribeTokenRefresh((token: string) => {
            originalRequest.headers.Authorization = `Bearer ${token}`
            resolve(api(originalRequest))
          })
        })
      }

      originalRequest._retry = true
      isRefreshing = true

      const refreshed = await auth.refreshAccessToken()

      if (refreshed) {
        const newToken = auth.accessToken!
        onRefreshComplete(newToken)
        originalRequest.headers.Authorization = `Bearer ${newToken}`
        isRefreshing = false
        return api(originalRequest)
      }

      isRefreshing = false
      refreshSubscribers = []
      router.push('/signin')
      return Promise.reject(error)
    }

    return Promise.reject(error)
  }
)
