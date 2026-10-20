<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<template>
  <a
    href="#main"
    class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-100 focus:rounded-md focus:bg-primary focus:px-4 focus:py-2 focus:text-sm focus:text-white"
  >
    Skip to main content
  </a>
  <RouterView />
</template>

<style scoped></style>
