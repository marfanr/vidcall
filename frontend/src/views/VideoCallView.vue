<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import CallControls from '@/components/video-call/CallControls.vue'
import CallHeader from '@/components/video-call/CallHeader.vue'
import CallStatusScreen from '@/components/video-call/CallStatusScreen.vue'
import FloatingReactions from '@/components/video-call/FloatingReactions.vue'
import JoinRequestToast from '@/components/video-call/JoinRequestToast.vue'
import VideoGrid from '@/components/video-call/VideoGrid.vue'
import { useVideoCallRoom } from '@/composables/useVideoCallRoom'
import WelcomeToast from '@/components/video-call/membersChangesToast.vue'

const route = useRoute()

const {
  participants,
  room,
  cameraOff,
  floating,
  currentRequest,
  remainingRequests,
  showReaction,
  resolveRequest,
  toggleCamera,
  hangup,
  memberChangesData,
  resolveNewMembers,
} = useVideoCallRoom(() => route.params.id)

const roomParamId = computed(() => String(route.params.id))

const roomStatus = computed<'denied' | 'waiting' | 'not-started' | 'not-found'>(() => {
  if (room.status === 'denied') return 'denied'
  if (room.status === 'waiting') return 'waiting'
  if (room.status === 'not-started') return 'not-started'
  return 'not-found'
})
</script>

<template>
  <div
    v-if="room.status === 'loading'"
    class="relative flex flex-col w-screen h-screen bg-gray-100 items-center justify-center"
  >
    Please Wait
  </div>

  <div v-else-if="room.status === 'joined'" class="flex flex-col w-screen h-screen bg-gray-100">
    <CallHeader :roomTitle="room.title" />
    <VideoGrid :participants="participants" />
    <FloatingReactions :reactions="floating" />

    <JoinRequestToast
      :request="currentRequest"
      :remaining="remainingRequests"
      @admit="resolveRequest('admit')"
      @deny="resolveRequest('deny')"
    />

    <WelcomeToast :user="memberChangesData" @resolve="resolveNewMembers" />

    <CallControls
      :camera-off="cameraOff"
      @toggle-camera="toggleCamera"
      @hangup="hangup"
      @select-reaction="showReaction"
    />
  </div>

  <!-- room exists, user is not in the room yet / room not found -->
  <CallStatusScreen v-else :status="roomStatus" :room-id="roomParamId" :room-owner="room.owner" />
</template>
