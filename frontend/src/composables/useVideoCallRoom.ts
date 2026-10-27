import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
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
  const onRemoteTrack = (e: RTCTrackEvent) => {
    const stream = e.streams[0] ?? new MediaStream([e.track])
    const streamId = stream.id
    console.log('onRemoteTrack event:', e.track.kind, e.streams, streamId)
    const target =
      participants.value.find((p) => p.conn_id && streamId.includes(p.conn_id)) ??
      participants.value.find((p) => p.id !== 0)

    if (target) {
      if (target.stream) {
        target.stream.addTrack(e.track)
        target.stream = new MediaStream(target.stream.getTracks())
      } else {
        target.stream = stream
      }
      if (e.track.kind === 'video') {
        target.cameraOff = false
      }
      if (e.track.kind === 'audio') {
        target.muted = false
      }
      e.track.onended = () => {
        if (e.track.kind === 'video' && target) {
          target.cameraOff = true
        }
        if (e.track.kind === 'audio' && target) {
          target.muted = true
        }
        console.log("ended track")
      }
    }
  }

  const handleControlChannel = (c: MessageEvent): void => {
    const data = JSON.parse(c.data)
    switch (data.type) {
      case 'track_state': {
        const target = participants.value.find((p) => p.conn_id === data.from)
        if (target && data.kind === 'video') target.cameraOff = data.muted
        if (target && data.kind === 'audio') target.muted = data.muted
        console.log('track state updated : ', target?.cameraOff, target?.muted)
        break
      }
      case 'track_ended': {
        const target = participants.value.find((p) => p.conn_id === c.data.from)
        if (target && c.data.kind === 'video') {
          target.cameraOff = true
          target.stream?.getVideoTracks().forEach((t) => t.stop())
        }
        break
      }
    }
  }

  const webrtc = useWebRTC(signaling.send, onRemoteTrack, handleControlChannel)
  const media = useLocalMedia(participants)

  // welcome members
  let welcomeSeq = 0
  const memberChangesStore = ref<MemberChanges[]>([])
  const memberChangesData = computed(() => memberChangesStore.value[0])
  const resolveNewMembers = () => {
    memberChangesStore.value.shift()
  }

  const cameraOff = computed(() => media.getMe()?.cameraOff ?? true)
  const muted = computed(() => media.getMe()?.muted ?? true)

  const onJoinedRoom = async (): Promise<void> => {
    const pc = webrtc.peerCon.value
    if (!pc) return

    const stream = media.getMe()?.stream
    if (stream) webrtc.addLocalStream(stream)

    const sdp = await webrtc.createOffer()
    if (sdp) signaling.sendOffer(sdp)
  }

  const renegotiate = async (): Promise<void> => {
    const pc = webrtc.peerCon.value
    if (!pc || pc.signalingState !== 'stable') return

    const sdp = await webrtc.createOffer()
    if (sdp) signaling.sendOffer(sdp)
  }

  const toggleCamera = async () => {
    await media.toggleCamera()

    const stream = media.getMe()?.stream
    if (!stream) return

    webrtc.addLocalStream(stream)
    await renegotiate()
  }

  const toggleMic = async () => {
    await media.toggleMic()

    const stream = media.getMe()?.stream
    if (!stream) return

    webrtc.addLocalStream(stream)
    await renegotiate()
  }

  watch(() => room.status === 'joined', onJoinedRoom)

  signaling.onMessage(async (msg) => {
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
          webrtc.sendDataControl('test')
          break

        // synchronize current members
        case 'sync':
          console.log(data)
          const members: {
            conn_id: string
            username: string
          }[] = data.members
          members.forEach((v) => {
            participants.value.push({
              id: ++participantSeq,
              conn_id: v.conn_id,
              name: v.username,
              muted: true,
              cameraOff: true,
              stream: null,
            })
          })

          break

        case 'request': {
          const { username, conn_id } = data.from
          requests.addRequest({ username, conn_id })
          break
        }

        case 'offer': {
          const answerSdp = await webrtc.handleRemoteOffer(data.sdp)
          if (answerSdp) {
            signaling.sendAnswer(answerSdp)
          }
          break
        }

        case 'answer': {
          console.log('answer', data)
          await webrtc.setRemoteAnswer(data.sdp)
          break
        }

        case 'ice': {
          webrtc.addIceCandidate({
            candidate: data.candidate,
            sdpMid: data.sdpMid,
            sdpMLineIndex: data.sdpMLineIndex,
            usernameFragment: data.usernameFragment,
          })
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
      await webrtc.init()
    } catch (error) {
      console.error('webrtc init failed:', error)
      room.status = 'not-found'
      return
    }

    try {
      const res = await api.get(`/room/${getRoomParam()}`)
      room.id = res.data.id
      room.owner = res.data.owner
      room.status = res.data.is_started ? 'waiting' : 'not-started'
      room.title = res.data.title ?? 'Untitled room'

      await media.init()

      signaling.join(res.data.id, auth.accessToken)
    } catch (error) {
      room.status = 'not-found'
      console.error(error)
    }
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
    toggleCamera,
    toggleMic,
    muted,
    hangup,
    memberChangesData,
    resolveNewMembers,
    sendControl: webrtc.sendDataControl,
  }
}
