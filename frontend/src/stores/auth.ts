import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/lib/axios'

export const useAuthStore = defineStore('auth', () => {
  const accessToken = ref<string | null>(null)
  const refreshToken = ref<string | null>(null)
  const authenticated = ref(false)

  function setTokens(access: string, refresh: string) {
    accessToken.value = access
    refreshToken.value = refresh
    authenticated.value = true
  }

  function setAccessToken(token: string) {
    accessToken.value = token
    authenticated.value = true
  }

  async function refreshAccessToken(): Promise<boolean> {
    if (!refreshToken.value) return false
    try {
      const res = await api.post('/auth/refresh_token', {
        refreshToken: refreshToken.value,
      })
      const newAccessToken = res.data?.token
      if (newAccessToken) {
        accessToken.value = newAccessToken
        return true
      }
      return false
    } catch {
      clearAuth()
      return false
    }
  }

  function clearAuth() {
    accessToken.value = null
    refreshToken.value = null
    authenticated.value = false
  }

  return {
    accessToken,
    refreshToken,
    authenticated,
    setTokens,
    setAccessToken,
    refreshAccessToken,
    clearAuth,
  }
})
