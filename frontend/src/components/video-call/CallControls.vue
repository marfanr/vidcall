<script setup lang="ts">
import {
  CameraIcon,
  CameraOffIcon,
  CircleDotIcon,
  MessageSquarePlusIcon,
  MicIcon,
  MicOffIcon,
  PhoneOffIcon,
  ScreenShareIcon,
  UsersIcon,
} from 'lucide-vue-next'
import ReactionPicker from './ReactionPicker.vue'
import type { DataControlType } from '@/composables/useWebRTC.ts'

defineProps<{
  cameraOff: boolean
  muted: boolean
}>()

const emit = defineEmits<{
  'toggle-camera': []
  'toggle-mic': []
  'send-control': [msg: DataControlType]
  hangup: []
  'select-reaction': [emoji: string]
}>()

const onRecordClick = (): void => {
  console.log('send')
  emit(
    'send-control',
    JSON.stringify({
      type: 'record',
    }),
  )
}
</script>

<template>
  <div
    class="flex flex-row items-center justify-start sm:justify-center gap-x-1 sm:gap-x-3 md:gap-x-4 p-1 sm:p-2 overflow-x-auto w-full max-w-full no-scrollbar touch-pan-x"
  >
    <button
      type="button"
      aria-label="Toggle microphone"
      title="Toggle microphone"
      class="min-w-[44px] min-h-[44px] flex items-center justify-center p-1.5 sm:p-2 rounded-full cursor-pointer opacity-60 shrink-0 touch-manipulation"
      style="touch-action: manipulation"
      @click="emit('toggle-mic')"
    >
      <MicOffIcon v-if="muted" class="w-5 h-5 sm:w-6 sm:h-6 text-danger" />
      <MicIcon v-else class="w-5 h-5 sm:w-6 sm:h-6 text-gray-800" />
    </button>
    <button
      type="button"
      disabled
      aria-label="Screen share (coming soon)"
      title="Screen share (coming soon)"
      class="min-w-[44px] min-h-[44px] flex items-center justify-center p-1.5 sm:p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60 shrink-0"
    >
      <ScreenShareIcon class="w-5 h-5 sm:w-6 sm:h-6" />
    </button>
    <button
      type="button"
      title="Camera"
      class="min-w-[44px] min-h-[44px] flex items-center justify-center p-1.5 sm:p-2 rounded-full text-stone-950 opacity-60 cursor-pointer shrink-0 touch-manipulation"
      style="touch-action: manipulation"
      aria-label="Toggle camera"
      @click="emit('toggle-camera')"
    >
      <CameraIcon v-if="!cameraOff" class="w-5 h-5 sm:w-6 sm:h-6" />
      <CameraOffIcon v-else class="w-5 h-5 sm:w-6 sm:h-6" />
    </button>
    <button
      type="button"
      aria-label="Leave call"
      title="Leave call"
      @click="emit('hangup')"
      class="min-h-[44px] px-4 sm:px-6 py-1.5 sm:py-2 bg-danger text-white rounded-xl cursor-pointer transition hover:brightness-90 active:brightness-90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-danger shrink-0 touch-manipulation flex items-center justify-center"
      style="touch-action: manipulation"
    >
      <PhoneOffIcon class="w-5 h-5 sm:w-6 sm:h-6" aria-hidden="true" />
    </button>
    <button
      type="button"
      aria-label="Recording"
      title="Recording"
      class="min-w-[44px] min-h-[44px] flex items-center justify-center
         p-1.5 sm:p-2 rounded-full
         cursor-pointer shrink-0"
      v-on:click="onRecordClick"
    >
      <CircleDotIcon class="w-5 h-5 sm:w-6 sm:h-6 text-gray-800" />
    </button>
    <button
      type="button"
      disabled
      aria-label="Chat (coming soon)"
      title="Chat (coming soon)"
      class="min-w-[44px] min-h-[44px] flex items-center justify-center p-1.5 sm:p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60 shrink-0"
    >
      <MessageSquarePlusIcon class="w-5 h-5 sm:w-6 sm:h-6" />
    </button>
    <button
      type="button"
      disabled
      aria-label="Participants (coming soon)"
      title="Participants (coming soon)"
      class="min-w-[44px] min-h-[44px] flex items-center justify-center p-1.5 sm:p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60 shrink-0"
    >
      <UsersIcon class="w-5 h-5 sm:w-6 sm:h-6" />
    </button>

    <!-- reactions + emoji picker -->
    <div class="shrink-0 relative z-50">
      <ReactionPicker @select="emit('select-reaction', $event)" />
    </div>
  </div>
</template>
