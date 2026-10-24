<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { SmilePlusIcon } from 'lucide-vue-next'
import type { EmojiExt } from 'vue3-emoji-picker'
import EmojiPicker from 'vue3-emoji-picker'
import 'vue3-emoji-picker/css'

const emit = defineEmits<{
  select: [emoji: string]
}>()

const pickerOpen = ref(false)
const pickerRef = ref<HTMLElement | null>(null)

const pickEmoji = (selected: EmojiExt) => {
  emit('select', selected.i)
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

onMounted(() => {
  document.addEventListener('pointerdown', onDocPointerDown)
  document.addEventListener('keydown', onDocKeydown)
})

onUnmounted(() => {
  document.removeEventListener('pointerdown', onDocPointerDown)
  document.removeEventListener('keydown', onDocKeydown)
})
</script>

<template>
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
</template>
