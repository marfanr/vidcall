<script setup lang="ts">
import { ref, watch, type ComponentPublicInstance } from 'vue'
import { MicIcon, MicOffIcon } from 'lucide-vue-next'
import type { Participant } from '@/types/video-call'

const props = defineProps<{
  participant: Participant
  tileClass?: string
}>()

const videoRef = ref<HTMLVideoElement | null>(null)

const setStream = () => {
  if (videoRef.value && props.participant.stream) {
    videoRef.value.srcObject = props.participant.stream
  }
}

const setVideoRef = (el: Element | ComponentPublicInstance | null) => {
  videoRef.value = el instanceof HTMLVideoElement ? el : null
  if (videoRef.value) setStream()
}

watch(() => props.participant.stream, setStream)
</script>

<template>
  <div
    class="relative overflow-hidden rounded-sm min-w-0 min-h-0 h-full bg-stone-900 text-white flex items-center justify-center"
    :class="tileClass"
  >
    <!-- Avatar -->
    <div
      v-if="participant.cameraOff"
      class="size-24 shrink-0 rounded-full bg-blue-ribbon-800 flex items-center justify-center"
    >
      <span class="text-white text-2xl font-medium" aria-hidden="true">
        {{ participant.name[0]?.toUpperCase() }}
      </span>
    </div>

    <!-- Video -->
    <div v-else class="absolute inset-0">
      <video
        :ref="setVideoRef"
        class="w-full h-full bg-black object-cover aspect-video"
        autoplay
        playsinline
        :muted="participant.id === 0"
      ></video>
    </div>

    <!-- Participant info -->
    <div
      class="absolute bottom-4 left-4 p-2 text-xs flex flex-row gap-x-3 items-center bg-zinc-800/60 rounded-md text-white"
    >
      <span>{{ participant.name }}</span>
      <MicOffIcon :size="15" v-if="participant.muted" aria-label="Muted" />
      <MicIcon :size="15" v-else aria-label="Unmuted" />
    </div>
  </div>
</template>
