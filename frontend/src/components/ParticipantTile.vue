<script setup lang="ts">
import { computed } from 'vue'
import { MicIcon, MicOffIcon } from 'lucide-vue-next'

const props = defineProps<{
  name: string
  muted: boolean
  cameraOff: boolean
}>()

const initials = computed(() =>
  props.name
    .trim()
    .split(/\s+/)
    .map((w) => w[0])
    .slice(0, 2)
    .join('')
    .toUpperCase(),
)
</script>

<template>
  <div
    class="relative flex h-full w-full flex-col items-center justify-center gap-2 overflow-hidden rounded-lg bg-stone-900 text-white"
  >
    <span
      class="grid aspect-square w-1/3 min-w-10 max-w-28 place-items-center rounded-full bg-primary text-sm font-medium"
      aria-hidden="true"
    >
      {{ initials }}
    </span>
    <span v-if="cameraOff" class="text-xs text-stone-400">Camera off</span>

    <div
      class="absolute bottom-2 left-2 flex min-w-10 items-center justify-between gap-3 rounded bg-black/40 px-2 py-1 text-xs"
    >
      <span>{{ name }}</span>
      <MicOffIcon v-if="muted" :size="14" aria-label="Muted" />
      <MicIcon v-else :size="14" aria-label="Unmuted" />
    </div>
  </div>
</template>
