<script setup lang="ts">
import {
  CameraIcon,
  CameraOffIcon,
  MessageSquarePlusIcon,
  MicOffIcon,
  PhoneOffIcon,
  ScreenShareIcon,
  UsersIcon,
} from 'lucide-vue-next'
import ReactionPicker from './ReactionPicker.vue'

defineProps<{
  cameraOff: boolean
}>()

const emit = defineEmits<{
  'toggle-camera': []
  hangup: []
  'select-reaction': [emoji: string]
}>()
</script>

<template>
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
      aria-label="Screen share (coming soon)"
      title="Screen share (coming soon)"
      class="p-2 rounded-full text-gray-400 cursor-not-allowed opacity-60"
    >
      <ScreenShareIcon />
    </button>
    <button
      type="button"
      title="Camera"
      class="p-2 rounded-full text-stone-950 opacity-60 cursor-pointer"
      aria-label="Toggle camera"
      @click="emit('toggle-camera')"
    >
      <CameraIcon v-if="!cameraOff" />
      <CameraOffIcon v-else />
    </button>
    <button
      type="button"
      aria-label="Leave call"
      title="Leave call"
      @click="emit('hangup')"
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
    <ReactionPicker @select="emit('select-reaction', $event)" />
  </div>
</template>
