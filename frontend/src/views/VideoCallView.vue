<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { PhoneOffIcon } from 'lucide-vue-next'

const ROOM_ID = 'IOCz1Y20EJhPbf9QPowM8'

let localStream: MediaStream | null = null
const localVideoRef = ref<HTMLVideoElement | null>(null)
const remoteVideoRef = ref<HTMLVideoElement | null>(null)

const peerCon = new RTCPeerConnection({
  iceServers: [{ urls: 'stun:74.125.250.129:19302' }],
})
const socket = new WebSocket(`ws://${location.hostname}:8080/v1/room/ws`)

const pending: RTCIceCandidateInit[] = []
let remoteSet = false

const wsReady = new Promise<void>((resolve, reject) => {
  if (socket.readyState === WebSocket.OPEN) return resolve()
  socket.addEventListener('open', () => resolve(), { once: true })
  socket.addEventListener('error', () => reject(new Error('ws error')), { once: true })
})

const send = (obj: unknown) => socket.send(JSON.stringify(obj))

peerCon.addEventListener('icecandidate', (e) => {
  if (e.candidate) {
    console.log('local candidate:', e.candidate.candidate)
    send({
      type: 'candidate',
      candidate: e.candidate.candidate,
      mid: e.candidate.sdpMid,
      line_index: e.candidate.sdpMLineIndex,
      username_fragment: e.candidate.usernameFragment,
    })
  } else {
    console.log('local gathering done')
  }
})
peerCon.addEventListener('icecandidateerror', (e) => console.warn('ice candidate error', e))
peerCon.addEventListener('iceconnectionstatechange', () =>
  console.log('ice:', peerCon.iceConnectionState))
peerCon.addEventListener('icegatheringstatechange', () =>
  console.log('gather:', peerCon.iceGatheringState))
peerCon.addEventListener('connectionstatechange', () =>
  console.log('pc:', peerCon.connectionState))
peerCon.addEventListener('track', (e) => {
  console.log('remote track', e.track.kind)
  if (remoteVideoRef.value) remoteVideoRef.value.srcObject = e.streams[0] ?? new MediaStream([e.track])
})

socket.onmessage = async (msg) => {
  const data = JSON.parse(msg.data)
  console.log('ws in:', data.type, data)
  try {
    switch (data.type) {
      case 'answer':
        await peerCon.setRemoteDescription({ type: 'answer', sdp: data.sdp })
        remoteSet = true
        for (const c of pending) await peerCon.addIceCandidate(c)
        pending.length = 0
        break

      case 'ice': {
        const c: RTCIceCandidateInit = {
          candidate: data.candidate,
          sdpMid: data.sdpMid || '0',          // webrtc-rs sering mengirim sdpMid kosong
          sdpMLineIndex: data.sdpMLineIndex ?? 0,
          usernameFragment: data.usernameFragment || undefined,
        }
        if (remoteSet) await peerCon.addIceCandidate(c)
        else pending.push(c)
        break
      }
    }
  } catch (err) {
    console.error('signaling error:', data.type, err)
  }
}

onMounted(async () => {
  try {
    localStream = await navigator.mediaDevices.getUserMedia({ video: true, audio: false })

    if (localVideoRef.value) {
      localVideoRef.value.srcObject = localStream
      await localVideoRef.value.play()
    }

    for (const t of localStream.getTracks()) peerCon.addTrack(t, localStream)

    await wsReady
    send({ type: 'join', room: ROOM_ID })

    const offer = await peerCon.createOffer()
    await peerCon.setLocalDescription(offer)
    send({ type: 'offer', sdp: offer.sdp })
  } catch (error) {
    console.error(error)
  }
})

const hangup = () => {
  send({ type: 'leave' })
  peerCon.close()
  localStream?.getTracks().forEach((t) => t.stop())
}

onUnmounted(() => {
  localStream?.getTracks().forEach((t) => t.stop())
  peerCon.close()
  socket.close()
})
</script>

<style lang="css">
@import 'tailwindcss';
</style>

<template>
  <div class="flex flex-col w-screen h-screen">
    <div class="flex flex-row gap-x-4 items-center">
      <h2 class="text-3xl p-4 font-medium">VidCall</h2>
      <p class="text-xs p-2 border rounded-full border-gray-200 text-gray-400">
        room_id: {{ $route.params.id }}
      </p>
    </div>
    <div class="flex flex-wrap gap-2 p-4 h-3/4 w-full">
      <video class="flex w-70 h-70 rounded-lg bg-black" ref="localVideoRef" autoplay playsinline muted />
      <video class="flex w-70 h-70 rounded-lg bg-black" ref="remoteVideoRef" autoplay playsinline />
    </div>
    <div class="flex flex-row items-center justify-center p-2">
      <button class="px-6 py-2 bg-red-600 text-white rounded-xl cursor-pointer" @click="hangup">
        <PhoneOffIcon />
      </button>
    </div>
  </div>
</template>
