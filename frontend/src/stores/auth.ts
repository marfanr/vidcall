import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAuthStore = defineStore('auth', () => {
  const accessToken = ref<string | null>(null)
  const authenticated = ref(false)

  function setAccessToken(token: string) {
    accessToken.value = token
    authenticated.value = true
  }

  function clearAuth() {
    accessToken.value = null
    authenticated.value = false
  }

  return {
    accessToken,
    authenticated,
    setAccessToken,
    clearAuth,
  }
})
