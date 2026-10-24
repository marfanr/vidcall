import { computed, ref, watch } from 'vue'
import { useNotifySound } from '@/lib/audio'
import type { JoinRequest } from '@/types/video-call'

interface Options {
  getRoomId: () => string | undefined
  answerRequest: (room: string | undefined, connId: string) => void
}

export function useJoinRequests({ getRoomId, answerRequest }: Options) {
  const notifySound = useNotifySound()
  const joinRequests = ref<JoinRequest[]>([])
  const currentRequest = computed(() => joinRequests.value[0])
  const remainingRequests = computed(() => joinRequests.value.length - 1)

  watch(
    () => joinRequests.value.length,
    (len, prev) => {
      if (prev === 0 && len > 0) notifySound.play()
    },
  )

  const addRequest = (request: JoinRequest) => {
    if (!joinRequests.value.some((r) => r.conn_id === request.conn_id)) {
      joinRequests.value.push(request)
    }
  }

  const cancelRequest = (connId: string) => {
    joinRequests.value = joinRequests.value.filter((r) => r.conn_id !== connId)
  }

  const resolveRequest = (type: 'admit' | 'deny') => {
    const req = currentRequest.value
    if (!req) return
    // TODO!: handle deny
    if (type == 'admit') {
      answerRequest(getRoomId(), req.conn_id)
    }
    joinRequests.value.shift()
  }

  return {
    joinRequests,
    currentRequest,
    remainingRequests,
    addRequest,
    cancelRequest,
    resolveRequest,
  }
}
