<script setup lang="ts">
import { computed } from 'vue'
import type { Participant } from '@/types/video-call'
import VideoTile from './VideoTile.vue'

const props = defineProps<{
  participants: Participant[]
}>()

const columns = computed(() => {
  const n = props.participants.length

  if (n <= 1) return 1
  if (n <= 2) return 2
  if (n <= 6) return 3
  return 4
})

const rows = computed(() => Math.ceil(props.participants.length / columns.value))

const participantStyle = () =>
  props.participants.length === 1
    ? 'w-full max-w-3xl aspect-video max-h-5/6'
    : props.participants.length <= 3
      ? 'w-full max-h-2/3'
      : 'w-full h-full'
</script>

<template>
  <main
    class="grid p-4 h-11/12 w-full items-center justify-center gap-2 overflow-hidden"
    :class="participants.length === 1 ? 'grid-cols-1 grid-rows-1 place-items-center' : ''"
    :style="{
      gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
      gridTemplateRows: `repeat(${rows}, minmax(0, 1fr))`,
    }"
  >
    <VideoTile
      v-for="p in participants"
      :key="p.id"
      :participant="p"
      :tile-class="participantStyle()"
    />
  </main>
</template>
