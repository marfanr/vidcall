<script setup lang="ts">
import { ref, onMounted, onUnmounted, type ComponentPublicInstance, computed } from 'vue'
import {
  CameraIcon,
  CameraOffIcon,
  Grid3X3Icon,
  MessageSquarePlusIcon,
  MicIcon,
  MicOffIcon,
  PhoneOffIcon,
  ScreenShareIcon,
  SmilePlusIcon,
  UsersIcon,
} from 'lucide-vue-next'
import EmojiPicker from 'vue3-emoji-picker'
import 'vue3-emoji-picker/css'
import type { EmojiExt } from 'vue3-emoji-picker'

const ROOM_ID = 'blWGZU0eG9h0'

const debug = (...args: unknown[]) => {
  if (import.meta.env.DEV) console.log(...args)
}

let localStream: MediaStream | null = null
// const localVideoRef = ref<HTMLVideoElement | null>(null)
const remoteVideoRef = ref<HTMLVideoElement | null>(null)

const peerCon = new RTCPeerConnection({
  iceServers: [{ urls: 'stun:74.125.250.129:19302' }],
})
const socket = new WebSocket(`ws://${location.hostname}:8080/v1/room/ws`)

const pending: RTCIceCandidateInit[] = []
let remoteSet = false

// const wsReady = new Promise<void>((resolve, reject) => {
//   if (socket.readyState === WebSocket.OPEN) return resolve()
//   socket.addEventListener('open', () => resolve(), { once: true })
//   socket.addEventListener('error', () => reject(new Error('ws error')), { once: true })
// })

const send = (obj: unknown) => socket.send(JSON.stringify(obj))

interface FloatingReaction {
  key: number
  emoji: string
  left: number
}

const pickerOpen = ref(false)
const pickerRef = ref<HTMLElement | null>(null)
const floating = ref<FloatingReaction[]>([])
let reactionSeq = 0

const cameraOn = ref(false)
const micOn = ref(false)

interface Participant {
  id: number
  name: string
  muted: boolean
  cameraOff: boolean
  stream: MediaStream | null
}

// 0 is local/own/me
const participants = ref<Participant[]>([
  {
    id: 0,
    name: 'me',
    muted: true,
    cameraOff: true,
    stream: null,
  },
  {
    id: 1,
    name: 'Hashim',
    muted: true,
    cameraOff: true,
    stream: null,
  },
  // {
  //   id: 2,
  //   name: 'Hashim',
  //   muted: true,
  //   cameraOff: true,
  //   stream: null,
  // },
  // {
  //   id: 3,
  //   name: 'Hashim',
  //   muted: true,
  //   cameraOff: true,
  //   stream: null,
  // },
])

const getMe = () => participants.value.find((p) => p.id == 0)

const showReaction = (emoji: string) => {
  const key = ++reactionSeq
  floating.value.push({ key, emoji, left: 10 + Math.random() * 80 })
  setTimeout(() => {
    floating.value = floating.value.filter((r) => r.key !== key)
  }, 2500)
}

const pickEmoji = (selected: EmojiExt) => {
  const emoji = selected.i
  showReaction(emoji)
  if (socket.readyState === WebSocket.OPEN) send({ type: 'reaction', emoji })
  pickerOpen.value = false
}

const onDocPointerDown = (e: PointerEvent) => {
  if (pickerOpen.value && pickerRef.value && !pickerRef.value.contains(e.target as Node)) {
    pickerOpen.value = false
  }
}
const onDocKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') pickerOpen.value = false
}

/* ---------- WebRTC ---------- */
// will be moved
peerCon.addEventListener('icecandidate', (e) => {
  if (e.candidate) {
    // debug('local candidate:', e.candidate.candidate)
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
// peerCon.addEventListener('iceconnectionstatechange', () =>
//   // debug('ice:', peerCon.iceConnectionState),
// )
// peerCon.addEventListener('icegatheringstatechange', () =>
//   // debug('gather:', peerCon.iceGatheringState),
// )
peerCon.addEventListener('connectionstatechange', () => debug('pc:', peerCon.connectionState))
peerCon.addEventListener('track', (e) => {
  debug('remote track', e.track.kind)
  if (remoteVideoRef.value)
    remoteVideoRef.value.srcObject = e.streams[0] ?? new MediaStream([e.track])
})

socket.onmessage = async (msg) => {
  const data = JSON.parse(msg.data)
  // debug('ws in:', data.type, data)
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
          sdpMid: data.sdpMid || '0', // webrtc-rs sering mengirim sdpMid kosong
          sdpMLineIndex: data.sdpMLineIndex ?? 0,
          usernameFragment: data.usernameFragment || undefined,
        }
        if (remoteSet) await peerCon.addIceCandidate(c)
        else pending.push(c)
        break
      }

      case 'reaction':
        if (typeof data.emoji === 'string') showReaction(data.emoji)
        break
    }
  } catch (err) {
    console.error('signaling error:', data.type, err)
  }
}

onMounted(async () => {
  document.addEventListener('pointerdown', onDocPointerDown)
  document.addEventListener('keydown', onDocKeydown)

  try {
    const me = getMe()
    if (!me?.cameraOff || micOn.value) {
      localStream = await navigator.mediaDevices.getUserMedia({
        video: cameraOn.value,
        audio: micOn.value,
      })
    }
    // const audio = localStream.getAudioTracks()[0]
    //     // TODO: read option
    //     if (video) {
    //       video.stop()
    //     }
    //     if (audio) audio.enabled = false
    //     if (localVideoRef.value) {
    //       localVideoRef.value.srcObject = localStream
    //       await localVideoRef.value.play()
    //     }
    //     for (const t of localStream.getTracks()) peerCon.addTrack(t, localStream)
    //     await wsReady
    //     send({ type: 'join', room: ROOM_ID })
    //     const offer = await peerCon.createOffer()
    //     await peerCon.setLocalDescription(offer)
    //     send({ type: 'offer', sdp: offer.sdp })
  } catch (error) {
    console.error(error)
  }
})

const hangup = () => {
  send({ type: 'leave' })
  peerCon.close()
  // localStream?.getTracks().forEach((t) => t.stop())
}

const onCameraOnTogle = async () => {
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

onUnmounted(() => {
  document.removeEventListener('pointerdown', onDocPointerDown)
  document.removeEventListener('keydown', onDocKeydown)
  // localStream?.getTracks().forEach((t) => t.stop())
  peerCon.close()
  socket.close()
})

const setVideoRef = (p: Participant, el: Element | ComponentPublicInstance | null) => {
  if (!(el instanceof HTMLVideoElement)) return
  if (p.stream) {
    el.srcObject = p.stream
  }
}

const columns = computed(() => {
  const n = participants.value.length

  if (n <= 1) return 1
  if (n <= 2) return 2
  if (n <= 6) return 3
  if (n <= 12) return 4
  return 4
})

const rows = computed(() => Math.ceil(participants.value.length / columns.value))
const participantStyle = () =>
  participants.value.length === 1
    ? 'w-full max-w-3xl aspect-video max-h-full'
    : participants.value.length <= 3
      ? 'w-full max-h-2/3'
      : 'w-full h-full'
</script>

<template>
  <div class="relative flex flex-col w-screen h-screen bg-gray-100">
    <header class="flex">
      <ul class="w-full flex flex-row items-center px-6 py-1 h-full justify-between gap-1">
        <li>
          <h1 class="p-2 bg-stone-100/20 text-base font-medium text-gray-950">
            {{ ROOM_ID }}
          </h1>
        </li>
        <li>
          <button
            type="button"
            class="flex flex-row gap-x-1 items-center cursor-pointer rounded-md px-2 py-1 text-sm text-gray-700 transition hover:bg-gray-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          >
            <Grid3X3Icon :size="18" aria-hidden="true" /> View
          </button>
        </li>
      </ul>
    </header>

    <!-- body -->
    <div
      class="grid p-4 h-11/12 w-full items-center justify-center gap-2 overflow-hidden"
      :class="participants.length === 1 ? 'grid-cols-1 grid-rows-1 place-items-center' : ''"
      :style="{
        gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
        gridTemplateRows: `repeat(${rows}, minmax(0, 1fr))`,
      }"
    >
      <div
        v-for="p in participants"
        :key="p.id"
        class="relative overflow-hidden rounded-sm min-w-0 min-h-0 h-full bg-stone-900 text-white flex items-center justify-center"
        :class="participantStyle()"
      >
        <!-- Avatar -->
        <div
          v-if="p.cameraOff"
          class="size-24 shrink-0 rounded-full bg-blue-ribbon-800 flex items-center justify-center"
        >
          <span class="text-white text-2xl font-medium" aria-hidden="true">
            {{ p.name[0]?.toUpperCase() }}
          </span>
        </div>

        <!-- Video -->
        <div v-else class="absolute inset-0">
          <video
            class="w-full h-full bg-black object-cover aspect-video"
            autoplay
            playsinline
            :ref="(e) => setVideoRef(p, e)"
          ></video>
        </div>

        <!-- Participant info -->
        <div
          class="absolute bottom-4 left-4 p-2 text-xs flex flex-row gap-x-3 items-center bg-zinc-800/60 rounded-md text-white"
        >
          <span>{{ p.name }}</span>
          <MicOffIcon :size="15" v-if="p.muted" aria-label="Muted" />
          <MicIcon :size="15" v-else aria-label="Unmuted" />
        </div>
      </div>
    </div>

    <!-- floating reactions -->
    <div
      class="pointer-events-none absolute inset-x-0 bottom-20 h-1/2 overflow-hidden"
      aria-hidden="true"
    >
      <span
        v-for="r in floating"
        :key="r.key"
        class="reaction-float absolute bottom-0 text-4xl"
        :style="{ left: r.left + '%' }"
      >
        {{ r.emoji }}
      </span>
    </div>

    <div class="flex flex-row items-center justify-center p-2 gap-x-4">
      <button
        type="button"
        disabled
        aria-label="Microphone (coming soon)"
        title="Microphone (coming soon)"
        class="p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60"
      >
        <MicOffIcon />
      </button>
      <button
        type="button"
        disabled
        aria-label="Microphone"
        title="Microphone (coming soon)"
        class="p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60"
      >
        <ScreenShareIcon />
      </button>
      <button
        type="button"
        title="Camera"
        class="p-2 rounded-full text-stone-950 opacity-60 cursor-pointer"
        aria-label="turn of camera"
        @click="onCameraOnTogle"
      >
        <CameraIcon v-if="!getMe()?.cameraOff" />
        <CameraOffIcon v-else />
      </button>
      <button
        type="button"
        aria-label="Leave call"
        title="Leave call"
        @click="hangup"
        class="px-6 py-2 bg-danger text-white rounded-xl cursor-pointer transition hover:brightness-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger"
      >
        <PhoneOffIcon aria-hidden="true" />
      </button>
      <button
        type="button"
        disabled
        aria-label="Chat (coming soon)"
        title="Chat (coming soon)"
        class="p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60"
      >
        <MessageSquarePlusIcon />
      </button>
      <button
        type="button"
        disabled
        aria-label="Participants (coming soon)"
        title="Participants (coming soon)"
        class="p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60"
      >
        <UsersIcon />
      </button>

      <!-- reactions + emoji picker -->
      <div ref="pickerRef" class="relative">
        <button
          type="button"
          aria-label="Reactions"
          title="Reactions"
          aria-haspopup="true"
          :aria-expanded="pickerOpen"
          @click="pickerOpen = !pickerOpen"
          class="p-2 rounded-full text-gray-700 cursor-pointer transition hover:bg-gray-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          :class="{ 'bg-gray-200': pickerOpen }"
        >
          <SmilePlusIcon />
        </button>

        <div v-if="pickerOpen" class="absolute bottom-full right-0 mb-3 z-10">
          <EmojiPicker :native="true" theme="light" @select="pickEmoji" />
        </div>
      </div>
    </div>
  </div>
</template>

<style lang="css">
@import 'tailwindcss';

@keyframes reaction-float {
  0% {
    transform: translateY(0) scale(0.6);
    opacity: 0;
  }
  15% {
    transform: translateY(-40px) scale(1.2);
    opacity: 1;
  }
  100% {
    transform: translateY(-40vh) scale(1);
    opacity: 0;
  }
}
.reaction-float {
  animation: reaction-float 2.5s ease-out forwards;
}
@media (prefers-reduced-motion: reduce) {
  .reaction-float {
    animation-duration: 0.01s;
  }
}
</style>
