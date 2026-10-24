<script setup lang="ts">
import { computed, ref, useId } from 'vue'

interface Props {
  label: string
  type?: string
  placeholder?: string
  autocomplete?: string
  error?: string
  id?: string
}
const showPassword = ref(false)
const props = withDefaults(defineProps<Props>(), {
  type: 'text',
  placeholder: '',
  autocomplete: undefined,
  error: '',
  id: undefined,
})

const model = defineModel<string>({ default: '' })

const uid = useId()
const inputId = computed(() => props.id ?? `field-${uid}`)
const errorId = computed(() => `${inputId.value}-error`)
</script>

<template>
  <div>
    <label :for="inputId" class="sr-only">{{ label }}</label>
    <input
      :id="inputId"
      v-model="model"
      :type="type"
      v-if="type != 'password'"
      :autocomplete="autocomplete"
      :placeholder="placeholder || label"
      :aria-invalid="!!error"
      :aria-describedby="error ? errorId : undefined"
      class="h-11 w-full rounded-lg border border-slate-200 bg-white px-4 text-sm placeholder:text-slate-500 transition focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20"
    />
    <div class="relative" v-else>
      <input
        id="password"
        v-model="model"
        :type="showPassword ? 'text' : 'password'"
        autocomplete="current-password"
        placeholder="Password"
        :aria-invalid="!!error"
        :aria-describedby="error ? 'password-error' : undefined"
        class="h-11 w-full rounded-lg border border-slate-200 bg-white pl-4 pr-11 text-sm placeholder:text-slate-500 transition focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20"
      />

      <button
        type="button"
        class="absolute inset-y-0 right-0 flex w-11 items-center justify-center text-slate-500 hover:text-slate-700 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary"
        :aria-label="showPassword ? 'Hide password' : 'Show password'"
        @click="showPassword = !showPassword"
      >
        <svg
          v-if="!showPassword"
          class="size-4"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M2 12s3.500-7 10-7 10 7 10 7-3.500 7-10 7S2 12 2 12z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
        <svg
          v-else
          class="size-4"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path
            d="M17.940 17.940A10.940 10.940 0 0 1 12 19c-6.500 0-10-7-10-7a18.500 18.500 0 0 1 5.060-5.940M9.900 4.240A9.120 9.120 0 0 1 12 5c6.500 0 10 7 10 7a18.500 18.500 0 0 1-2.160 3.190M1 1l22 22"
          />
        </svg>
      </button>
    </div>
    <p v-if="error" :id="errorId" class="mt-1 text-xs text-red-600">
      {{ error }}
    </p>
  </div>
</template>
