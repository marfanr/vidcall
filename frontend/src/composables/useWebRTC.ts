import { ref } from 'vue'

export function useWebRTC(send: (obj: unknown) => unknown) {
  const debug = (...args: unknown[]) => {
    if (import.meta.env.DEV) console.log(...args)
  }

  const remoteVideoRef = ref<HTMLVideoElement | null>(null)

  // TODO: move to .env
  const peerCon = new RTCPeerConnection({
    iceServers: [{ urls: 'stun:74.125.250.129:19302' }],
  })

  peerCon.addEventListener('icecandidate', (e) => {
    if (e.candidate) {
      send({
        type: 'candidate',
        candidate: e.candidate.candidate,
        mid: e.candidate.sdpMid,
        line_index: e.candidate.sdpMLineIndex,
        username_fragment: e.candidate.usernameFragment,
      })
    } else {
      debug('local gathering done')
    }
  })
  peerCon.addEventListener('icecandidateerror', (e) => console.warn('ice candidate error', e))
  peerCon.addEventListener('connectionstatechange', () => debug('pc:', peerCon.connectionState))
  peerCon.addEventListener('track', (e) => {
    debug('remote track', e.track.kind)
    if (remoteVideoRef.value)
      remoteVideoRef.value.srcObject = e.streams[0] ?? new MediaStream([e.track])
  })

  const close = () => peerCon.close()

  return { remoteVideoRef, peerCon, close }
}
