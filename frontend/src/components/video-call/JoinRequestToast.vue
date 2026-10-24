<script setup lang="ts">
import type { JoinRequest } from '@/types/video-call'

defineProps<{
  request?: JoinRequest
  remaining: number
}>()

const emit = defineEmits<{
  admit: []
  deny: []
}>()
</script>

<template>
  <!-- join request toast-->
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
        v-if="request"
        :key="request.conn_id"
        role="status"
        aria-live="polite"
        class="fixed bottom-16 md:bottom-4 right-4 z-99 w-96 max-w-[calc(100vw-2rem)] rounded-xl border border-gray-200/80 bg-white p-4 shadow-xl ring-1 ring-black/5"
      >
        <div class="flex items-start gap-3">
          <div
            class="flex size-10 shrink-0 items-center justify-center rounded-full bg-indigo-50 text-sm font-semibold text-indigo-600 ring-1 ring-indigo-100"
          >
            {{ request.username[0]?.toUpperCase() }}
          </div>

          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-medium text-gray-900">
              {{ request.username }}
            </p>
            <p class="mt-0.5 text-sm text-gray-500">wants to join this meeting</p>

            <div class="mt-3 flex items-center gap-2">
              <button
                type="button"
                class="rounded-lg bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white shadow-xs transition hover:bg-indigo-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500 focus-visible:ring-offset-2"
                @click="emit('admit')"
              >
                Admit
              </button>
              <button
                type="button"
                class="rounded-lg border border-gray-200 bg-white px-3 py-1.5 text-sm font-medium text-gray-700 shadow-xs transition hover:bg-gray-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-gray-400"
                @click="emit('deny')"
              >
                Deny
              </button>
              <span v-if="remaining > 0" class="ml-auto text-xs text-gray-500">
                +{{ remaining }} more
              </span>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
