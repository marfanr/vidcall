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
    class="relative overflow-hidden rounded-sm min-w-0 min-h-0 h-full w-full bg-stone-900 text-white flex items-center justify-center"
    :class="tileClass"
  >
    <!-- Avatar -->
    <div
      v-if="participant.cameraOff"
      class="size-16 sm:size-20 md:size-24 shrink-0 rounded-full bg-blue-ribbon-800 flex items-center justify-center"
    >
      <span class="text-white text-lg sm:text-xl md:text-2xl font-medium" aria-hidden="true">
        {{ participant.name[0]?.toUpperCase() }}
      </span>
    </div>

    <!-- Video -->
    <div v-else class="absolute inset-0">
      <video
        :ref="setVideoRef"
        class="w-full h-full bg-black object-cover"
        autoplay
        playsinline
        :muted="participant.id === 0"
      ></video>
    </div>

    <!-- Participant info -->
    <div
      class="absolute bottom-1 left-1 sm:bottom-2 sm:left-2 md:bottom-4 md:left-4 p-1 sm:p-1.5 md:p-2 text-[10px] sm:text-xs flex flex-row gap-x-1.5 sm:gap-x-2 md:gap-x-3 items-center bg-zinc-800/60 rounded-md text-white max-w-[calc(100%-0.5rem)] sm:max-w-[calc(100%-1rem)]"
    >
      <span class="truncate">{{ participant.name }}</span>
      <MicOffIcon :size="12" class="sm:hidden" v-if="participant.muted" aria-label="Muted" />
      <MicIcon :size="12" class="sm:hidden" v-else aria-label="Unmuted" />
      <MicOffIcon :size="14" class="hidden sm:block md:hidden" v-if="participant.muted" aria-label="Muted" />
      <MicIcon :size="14" class="hidden sm:block md:hidden" v-else aria-label="Unmuted" />
      <MicOffIcon :size="15" class="hidden md:block" v-if="participant.muted" aria-label="Muted" />
      <MicIcon :size="15" class="hidden md:block" v-else aria-label="Unmuted" />
    </div>
  </div>
</template>
