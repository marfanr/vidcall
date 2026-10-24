<script setup lang="ts">
import type { MemberChanges } from '@/types/video-call'
import { ref, watch, onBeforeUnmount } from 'vue'

const props = withDefaults(
  defineProps<{
    user?: MemberChanges
    duration?: number
  }>(),
  { duration: 2000 },
)

const emit = defineEmits<{
  resolve: []
}>()

const visible = ref(false)
let timer: ReturnType<typeof setTimeout> | undefined

function clearTimer() {
  if (timer !== undefined) {
    clearTimeout(timer)
    timer = undefined
  }
}

watch(
  () => props.user?.eventId,
  (eventId) => {
    clearTimer()
    if (eventId === undefined) {
      visible.value = false
      return
    }
    visible.value = true
    timer = setTimeout(() => {
      timer = undefined
      visible.value = false
      emit('resolve')
    }, props.duration)
  },
  { immediate: true },
)

onBeforeUnmount(clearTimer)
</script>

<template>
  <Teleport to="body">
    <Transition
      mode="out-in"
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="translate-x-4 opacity-0"
      enter-to-class="translate-x-0 opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="translate-x-0 opacity-100"
      leave-to-class="translate-x-4 opacity-0"
    >
      <div
        v-if="visible && user"
        :key="user.eventId"
        role="status"
        aria-live="polite"
        class="fixed top-12 right-4 z-99 w-96 max-w-[calc(100vw-2rem)] rounded-xl border border-gray-200/80 bg-white p-4 shadow-xl ring-1 ring-black/5"
      >
        <div class="flex items-start gap-3">
          {{ user.username }}
          {{ user.type == 'join' ? 'has joined' : 'has leave' }}
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
