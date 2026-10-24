<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { Grid3X3Icon } from 'lucide-vue-next'

const props = defineProps<{
  roomTitle: string
}>()

const emit = defineEmits<{
  'update:roomTitle': [value: string]
}>()

const editing = ref(false)
const draft = ref(props.roomTitle)
const displayTitle = ref(props.roomTitle)
const inputRef = ref<HTMLInputElement | null>(null)

const inputWidth = computed(() => `${Math.max(draft.value.length + 1, 12)}ch`)

watch(
  () => props.roomTitle,
  (value) => {
    displayTitle.value = value
    if (!editing.value) draft.value = value
  },
)

const startEditing = async () => {
  if (editing.value) return

  draft.value = displayTitle.value
  editing.value = true

  await nextTick()
  const input = inputRef.value
  if (!input) return

  input.focus()
  input.setSelectionRange(input.value.length, input.value.length)
}

const commit = () => {
  if (!editing.value) return

  editing.value = false
  const value = draft.value.trim()

  if (!value) {
    draft.value = displayTitle.value
    return
  }

  if (value === displayTitle.value) return

  displayTitle.value = value
  emit('update:roomTitle', value)
}

const cancel = () => {
  editing.value = false
  draft.value = displayTitle.value
}
</script>

<template>
  <header class="flex w-full">
    <ul class="w-full flex flex-row items-center px-2 sm:px-6 py-1 justify-between gap-1 min-w-0">
      <li class="min-w-0 flex-1" title="Double-click to rename" @dblclick="startEditing">
        <input
          v-if="editing"
          ref="inputRef"
          v-model="draft"
          type="text"
          aria-label="Room name"
          :style="{ width: inputWidth }"
          class="p-1 sm:p-2 bg-stone-100/20 text-sm sm:text-base font-medium text-gray-950 outline-none border-b border-black max-w-full"
          @blur="commit"
          @keydown.enter.prevent="commit"
          @keydown.esc.prevent="cancel"
        />
        <h1 v-else class="p-1 sm:p-2 bg-stone-100/20 text-sm sm:text-base font-medium text-gray-950 select-none truncate max-w-full">
          {{ displayTitle }}
        </h1>
      </li>
      <li class="shrink-0">
        <button
          type="button"
          class="flex flex-row gap-x-1 items-center cursor-pointer rounded-md px-2 py-1 text-xs sm:text-sm text-gray-700 transition hover:bg-gray-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        >
          <Grid3X3Icon :size="16" class="sm:hidden" aria-hidden="true" />
          <Grid3X3Icon :size="18" class="hidden sm:block" aria-hidden="true" />
          <span class="hidden xs:inline sm:inline">View</span>
        </button>
      </li>
    </ul>
  </header>
</template>
