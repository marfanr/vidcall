export interface Participant {
  id: number
  conn_id: string | undefined
  name: string
  muted: boolean
  cameraOff: boolean
  stream: MediaStream | null
}

export interface JoinRequest {
  username: string
  conn_id: string
}

export interface FloatingReaction {
  key: number
  emoji: string
  left: number
}

export type RoomStatus = 'loading' | 'joined' | 'denied' | 'waiting' | 'not-started' | 'not-found'

export type MemberChangesType = 'join' | 'leave'

export interface MemberChanges {
  eventId: number
  username: string
  type: MemberChangesType
}
