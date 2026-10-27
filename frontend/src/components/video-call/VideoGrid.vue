<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
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
    ? 'w-full max-w-2xl aspect-video max-h-8/12'
    : props.participants.length <= 3
      ? 'w-full max-h-2/3'
      : 'w-full h-full'

const DESKTOP_QUERY = '(min-width: 768px)'

const gridRef = ref<HTMLElement | null>(null)
const lockedHeight = ref<number | null>(null)
const isDesktop = ref(false)

let resizeObserver: ResizeObserver | null = null
let mediaQuery: MediaQueryList | null = null
let lastWindowWidth = 0

const captureHeight = async () => {
  const el = gridRef.value

  if (!el || !isDesktop.value) {
    lockedHeight.value = null
    return
  }

  lockedHeight.value = null
  await nextTick()

  const natural = el.clientHeight
  lockedHeight.value = natural > 0 ? natural : null
}

const onMediaChange = (event: MediaQueryListEvent) => {
  isDesktop.value = event.matches
  captureHeight()
}

const onWindowResize = () => {
  if (window.innerWidth === lastWindowWidth) return
  lastWindowWidth = window.innerWidth
  captureHeight()
}

onMounted(() => {
  lastWindowWidth = window.innerWidth
  mediaQuery = window.matchMedia(DESKTOP_QUERY)
  isDesktop.value = mediaQuery.matches
  captureHeight()

  mediaQuery.addEventListener('change', onMediaChange)
  window.addEventListener('resize', onWindowResize)

  resizeObserver = new ResizeObserver(() => {
    const el = gridRef.value
    if (!el || !isDesktop.value) return

    const height = el.clientHeight
    if (lockedHeight.value !== null && height > lockedHeight.value) {
      lockedHeight.value = height
    }
  })

  if (gridRef.value) resizeObserver.observe(gridRef.value)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', onWindowResize)
  mediaQuery?.removeEventListener('change', onMediaChange)
  resizeObserver?.disconnect()
})
</script>

<template>
  <main class="w-full h-full min-h-0 overflow-x-hidden overflow-y-auto p-2 sm:p-4">
    <div
      ref="gridRef"
      class="grid w-full h-full items-center justify-center gap-1 sm:gap-2"
      :class="participants.length === 1 ? 'grid-cols-1 grid-rows-1 place-items-center' : ''"
      :style="{
        gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
        gridTemplateRows: `repeat(${rows}, minmax(0, 1fr))`,
        minHeight: lockedHeight === null ? undefined : `${lockedHeight}px`,
      }"
    >
      <VideoTile
        v-for="p in participants"
        :key="p.id"
        :participant="p"
        :tile-class="participantStyle()"
      />
    </div>
  </main>
</template>
