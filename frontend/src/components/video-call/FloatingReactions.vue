<script setup lang="ts">
import type { FloatingReaction } from '@/types/video-call'

defineProps<{
  reactions: FloatingReaction[]
}>()
</script>

<template>
  <div
    class="pointer-events-none absolute inset-x-0 bottom-20 h-1/2 overflow-hidden"
    aria-hidden="true"
  >
    <span
      v-for="r in reactions"
      :key="r.key"
      class="reaction-float absolute bottom-0 text-4xl"
      :style="{ left: r.left + '%' }"
    >
      {{ r.emoji }}
    </span>
  </div>
</template>

<style lang="css">
@keyframes reaction-float {
  0% {
    transform: translateY(0) scale(0.6);
    opacity: 0;
  }
  15% {
    transform: translateY(-40px) scale(1.2);
    opacity: 1;
  }
  100% {
    transform: translateY(-40vh) scale(1);
    opacity: 0;
  }
}
.reaction-float {
  animation: reaction-float 2.5s ease-out forwards;
}
@media (prefers-reduced-motion: reduce) {
  .reaction-float {
    animation-duration: 0.01s;
  }
}
</style>
