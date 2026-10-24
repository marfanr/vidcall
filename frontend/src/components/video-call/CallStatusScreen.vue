<script setup lang="ts">
defineProps<{
  status: 'denied' | 'waiting' | 'not-started' | 'not-found'
  roomId: string
  roomOwner?: string
}>()
</script>

<template>
  <div
    v-if="status === 'denied'"
    class="w-screen h-screen flex flex-col justify-center items-center gap-y-7"
  >
    <h2 class="text-xl">The host denied your request to join</h2>
    <h3 class="text-sm text-gray-600">{{ roomId }}</h3>
    <RouterLink
      to="/"
      class="rounded-md border border-gray-300 px-4 py-1.5 text-sm font-medium transition hover:bg-gray-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-400"
    >
      Home
    </RouterLink>
  </div>
  <div
    v-else-if="status === 'waiting'"
    class="w-screen h-screen flex flex-col justify-center items-center gap-y-7"
  >
    <h2 class="text-xl">Please wait, the meeting host will let you in</h2>
    <h3 class="text-sm text-gray-600">{{ roomId }}</h3>
    <div class="my-6" />
    <small>{{ roomOwner }}'s Meeting Room</small>
  </div>
  <div
    v-else-if="status === 'not-started'"
    class="w-screen h-screen flex flex-col justify-center items-center gap-y-7"
  >
    <h2 class="text-xl">The meeting has not started</h2>
    <h3 class="text-sm text-gray-600">{{ roomId }}</h3>
    <div class="my-6" />
    <small>{{ roomOwner }}'s Meeting Room</small>
  </div>
  <div v-else class="w-screen h-screen flex flex-col justify-center items-center gap-y-7">
    <h2 class="text-xl font-medium">The Meeting Room is not found</h2>
    <span class="text-sm">meeting id : {{ roomId }}</span>
    <p class="text-xs text-gray-800">going back to home</p>
    <RouterLink
      to="/"
      class="rounded-md border border-gray-300 px-4 py-1.5 text-sm font-medium transition hover:bg-gray-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-400"
    >
      Home
    </RouterLink>
  </div>
</template>
