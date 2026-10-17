<script setup lang="ts">
import { useAuthStore } from '@/stores/auth';
import { storeToRefs } from 'pinia';

const auth = useAuthStore();
const { accessToken } = storeToRefs(auth);

</script>

<template>
  {{ accessToken }}
</template>
