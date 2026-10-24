import { onUnmounted } from 'vue'
import { ReconnectWebsocket } from '@/lib/ws'

type MessageHandler = (msg: MessageEvent) => void

export function useSignaling() {
  const ws = new ReconnectWebsocket(`ws://${location.hostname}:8080/v1/ws`)
  ws.connect()

  const messageHandlers = new Set<MessageHandler>()

  // placeholder signaling for WebRTC
  const send = (obj: unknown) => obj

  const onMessage = (handler: MessageHandler) => {
    messageHandlers.add(handler)
    ws.on('message', handler)
  }

  const close = () => {
    for (const handler of messageHandlers) ws.off('message', handler)
    messageHandlers.clear()
    ws.close()
  }

  onUnmounted(close)

  const join = (room: string, token: string | null) => {
    ws.send({ type: 'join', room, token })
  }

  const answerRequest = (room: string | undefined, connId: string) => {
    ws.send({ type: 'answer_request', room_id: room, conn_id: connId })
  }

  const leave = (room: string | undefined) => {
    ws.send({ type: 'leave', room })
  }

  return { ws, send, onMessage, join, answerRequest, leave, close }
}
