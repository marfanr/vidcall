import { onBeforeUnmount } from 'vue'

export function useNotifySound() {
  const audio = new Audio('/ringtone/newNotify.mp3')
  audio.volume = 1

  async function play() {
    try {
      audio.currentTime = 0
      await audio.play()
    } catch {}
  }

  function stop() {
    audio.pause()
    audio.currentTime = 0
  }

  onBeforeUnmount(stop)

  return { play, stop }
}
