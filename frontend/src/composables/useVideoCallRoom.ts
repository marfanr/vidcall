import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { api } from '@/lib/axios'
import { useAuthStore } from '@/stores/auth'
import type { MemberChanges, Participant, RoomStatus } from '@/types/video-call'
import { useJoinRequests } from './useJoinRequests'
import { useLocalMedia } from './useLocalMedia'
import { useReactions } from './useReactions'
import { useSignaling } from './useSignaling'
import { useWebRTC } from './useWebRTC'

interface RoomState {
  id: string | undefined
  owner: string | undefined
  status: RoomStatus
  title: string
}

export function useVideoCallRoom(getRoomParam: () => string | string[] | undefined) {
  let participantSeq = 0
  const participants = ref<Participant[]>([
    {
      id: participantSeq,
      conn_id: undefined,
      name: 'me',
      muted: true,
      cameraOff: true,
      stream: null,
    },
  ])

  const room = reactive<RoomState>({
    id: undefined,
    owner: undefined,
    title: 'Untitled room',
    status: 'loading',
  })

  const reactions = useReactions()
  const signaling = useSignaling()
  const requests = useJoinRequests({
    getRoomId: () => room.id,
    answerRequest: signaling.answerRequest,
  })
  const webrtc = useWebRTC(signaling.send)
  const media = useLocalMedia(participants)

  // welcome members
  let welcomeSeq = 0
  const memberChangesStore = ref<MemberChanges[]>([])
  const memberChangesData = computed(() => memberChangesStore.value[0])
  const resolveNewMembers = () => {
    memberChangesStore.value.shift()
  }

  const cameraOff = computed(() => media.getMe()?.cameraOff ?? true)

  signaling.onMessage((msg) => {
    let type: string | undefined

    try {
      const data = JSON.parse(msg.data)
      type = data.type

      switch (data.type) {
        case 'reaction':
          if (typeof data.emoji === 'string') reactions.showReaction(data.emoji)
          break

        case 'join':
          if (data.accept) room.status = 'joined'
          else if (room.status !== 'joined') room.status = 'denied'
          break

        // synchronize current members
        case 'sync':
          break

        case 'request': {
          const { username, conn_id } = data.from
          requests.addRequest({ username, conn_id })
          break
        }

        case 'welcome': {
          const { username, conn_id } = data.from

          memberChangesStore.value.push({
            eventId: ++welcomeSeq,
            username,
            type: 'join',
          })

          if (!participants.value.some((p) => p.conn_id === conn_id)) {
            participants.value.push({
              id: ++participantSeq,
              conn_id,
              name: username,
              muted: true,
              cameraOff: true,
              stream: null,
            })
          }
          break
        }

        // guest disconnect
        // TODO: not handled on backend
        case 'request_cancel':
          requests.cancelRequest(data.conn_id)
          break

        case 'disconnect': {
          const idx = participants.value.findIndex((p) => p.conn_id === data.conn_id)
          if (idx !== -1) participants.value.splice(idx, 1)

          memberChangesStore.value.push({
            eventId: ++welcomeSeq,
            username: data.username,
            type: 'leave',
          })
          break
        }
      }
    } catch (err) {
      console.error('signaling error:', type, err)
    }
  })

  const hangup = () => {
    signaling.leave(room.id)
    webrtc.close()
  }

  onMounted(async () => {
    const auth = useAuthStore()

    try {
      const res = await api.get(`/room/${getRoomParam()}`)
      room.id = res.data.id
      room.owner = res.data.owner
      room.status = res.data.is_started ? 'waiting' : 'not-started'
      room.title = res.data.title ?? 'Untitled room'

      signaling.join(res.data.id, auth.accessToken)
    } catch (error) {
      room.status = 'not-found'
      console.error(error)
    }

    await media.init()
  })

  onUnmounted(() => {
    webrtc.close()
  })

  return {
    participants,
    room,
    cameraOff,
    floating: reactions.floating,
    currentRequest: requests.currentRequest,
    remainingRequests: requests.remainingRequests,
    showReaction: reactions.showReaction,
    resolveRequest: requests.resolveRequest,
    toggleCamera: media.toggleCamera,
    hangup,
    memberChangesData,
    resolveNewMembers,
  }
}
