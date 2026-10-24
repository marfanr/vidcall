<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { SmilePlusIcon } from 'lucide-vue-next'
import type { EmojiExt } from 'vue3-emoji-picker'
import EmojiPicker from 'vue3-emoji-picker'
import 'vue3-emoji-picker/css'

const emit = defineEmits<{
  select: [emoji: string]
}>()

const pickerOpen = ref(false)
const triggerRef = ref<HTMLElement | null>(null)
const panelRef = ref<HTMLElement | null>(null)
const isDesktop = ref(false)
const anchorStyle = ref<Record<string, string>>({})

const PICKER_GAP = 12
let mql: MediaQueryList | null = null

const onMqlChange = (e: MediaQueryListEvent) => {
  isDesktop.value = e.matches
}

const updatePosition = () => {
  const el = triggerRef.value
  if (!el) return
  const r = el.getBoundingClientRect()
  anchorStyle.value = {
    right: `${Math.max(window.innerWidth - r.right, 8)}px`,
    bottom: `${window.innerHeight - r.top + PICKER_GAP}px`,
  }
}

const panelStyle = computed(() => (isDesktop.value ? anchorStyle.value : undefined))

const toggle = () => {
  if (!pickerOpen.value) updatePosition()
  pickerOpen.value = !pickerOpen.value
}

const close = () => {
  pickerOpen.value = false
}

const pickEmoji = (selected: EmojiExt) => {
  emit('select', selected.i)
  close()
}

const onDocPointerDown = (e: PointerEvent) => {
  if (!pickerOpen.value) return
  const target = e.target as Node
  if (triggerRef.value?.contains(target)) return
  if (panelRef.value?.contains(target)) return
  close()
}

const onDocKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape') close()
}

onMounted(() => {
  mql = window.matchMedia('(min-width: 640px)')
  isDesktop.value = mql.matches
  mql.addEventListener('change', onMqlChange)
  document.addEventListener('pointerdown', onDocPointerDown)
  document.addEventListener('keydown', onDocKeydown)
  window.addEventListener('resize', updatePosition)
})

onUnmounted(() => {
  mql?.removeEventListener('change', onMqlChange)
  document.removeEventListener('pointerdown', onDocPointerDown)
  document.removeEventListener('keydown', onDocKeydown)
  window.removeEventListener('resize', updatePosition)
})
</script>

<template>
  <div ref="triggerRef" class="relative">
    <button
      type="button"
      aria-label="Reactions"
      title="Reactions"
      aria-haspopup="true"
      :aria-expanded="pickerOpen"
      @click="toggle"
      class="min-w-[44px] min-h-[44px] flex items-center justify-center p-1.5 sm:p-2 rounded-full text-gray-700 cursor-pointer transition hover:bg-gray-200 active:bg-gray-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary touch-manipulation"
      :class="{ 'bg-gray-200': pickerOpen }"
    >
      <SmilePlusIcon class="w-5 h-5 sm:w-6 sm:h-6" />
    </button>

    <Teleport to="body">
      <div
        v-if="pickerOpen"
        ref="panelRef"
        class="fixed inset-0 z-[9999] flex h-dvh flex-col bg-white sm:inset-auto sm:block sm:h-auto sm:bg-transparent"
        :style="panelStyle"
      >
        <!-- header mobile -->
        <div
          class="flex shrink-0 items-center justify-between border-b border-gray-200 bg-white px-4 pb-3 pt-[max(0.75rem,env(safe-area-inset-top))] sm:hidden"
        >
          <span class="text-base font-medium text-gray-800">Reactions</span>
          <button
            type="button"
            aria-label="Close"
            class="rounded-lg px-3 py-1.5 text-sm font-medium text-gray-700 active:bg-gray-100"
            @click="close"
          >
            Close
          </button>
        </div>

        <div
          class="min-h-0 flex-1 bg-white pb-[env(safe-area-inset-bottom)] sm:bg-transparent sm:pb-0"
        >
          <EmojiPicker
            :native="true"
            theme="light"
            :hide-search="true"
            @select="pickEmoji"
            class="h-full w-full max-w-none rounded-none shadow-none sm:h-[450px] sm:w-[325px] sm:rounded-lg sm:shadow-xl"
          />
        </div>
      </div>
    </Teleport>
  </div>
</template>
