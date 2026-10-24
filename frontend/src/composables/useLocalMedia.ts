import { ref, type Ref } from 'vue'
import type { Participant } from '@/types/video-call'

export function useLocalMedia(participants: Ref<Participant[]>) {
  let localStream: MediaStream | null = null

  const cameraOn = ref(false)
  const micOn = ref(false)

  const getMe = () => participants.value.find((p) => p.id == 0)

  const init = async () => {
    try {
      const me = getMe()
      if (!me?.cameraOff || micOn.value) {
        localStream = await navigator.mediaDevices.getUserMedia({
          video: cameraOn.value,
          audio: micOn.value,
        })
      }
    } catch (error) {
      console.error(error)
    }
  }

  const toggleCamera = async () => {
    const me = getMe()
    if (!me) return

    if (!me.cameraOff) {
      localStream?.getVideoTracks().forEach((t) => {
        t.stop()
        localStream?.removeTrack(t)
      })
      me.cameraOff = true
      return
    }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: true,
        audio: false,
      })

      const videoTrack = stream.getVideoTracks()[0]

      if (videoTrack) {
        localStream ??= new MediaStream()
        localStream.addTrack(videoTrack)
        me.stream = localStream
      }

      me.cameraOff = false
    } catch (e) {
      console.log(e)
    }
  }

  return { cameraOn, micOn, init, toggleCamera, getMe }
}
