export interface ReconnectingWebSocketOptions {
  baseDelay: number
  maxDelay: number
  factor: number
  maxRetries: number
  heartbeatInterval: number
  heartbeatTimeout: number
}

const DEFAULT_OPTS: ReconnectingWebSocketOptions = {
  baseDelay: 1000,
  maxDelay: 30_000,
  factor: 3,
  maxRetries: 2,
  heartbeatInterval: 25_000,
  heartbeatTimeout: 10_000,
}

type Listener<T> = (data: T) => void
type Listeners = { [K in keyof WebSocketEventMap]: Listener<WebSocketEventMap[K]>[] }

export class ReconnectWebsocket {
  readonly url: string
  readonly protocols?: string
  readonly opts: Partial<ReconnectingWebSocketOptions>

  private ws: WebSocket | null = null
  private retries = 0
  private closedByUser: boolean = false
  private listeners: Listeners = {
    open: [],
    message: [],
    close: [],
    error: [],
  }
  private reconnectTimer?: number | undefined | null
  private isConnected: boolean = false
  private pendingMessage: (BufferSource | Blob | string | object)[] = []

  constructor(url: string, protocol?: string, opts: Partial<ReconnectingWebSocketOptions> = {}) {
    this.url = url
    this.protocols = protocol
    this.opts = { ...DEFAULT_OPTS, ...opts }
    window.addEventListener('online', this.onOnline)
  }

  private emit<K extends keyof WebSocketEventMap>(evt: K, data: WebSocketEventMap[K]): void {
    for (const fn of this.listeners[evt]) fn(data)
  }

  get isOpen(): boolean {
    return this.ws?.readyState === WebSocket.OPEN
  }

  on<K extends keyof WebSocketEventMap>(evt: K, fn: Listener<WebSocketEventMap[K]>): this {
    this.listeners[evt].push(fn)
    return this
  }

  off<K extends keyof WebSocketEventMap>(evt: K, fn: Listener<WebSocketEventMap[K]>): this {
    this.listeners[evt] = this.listeners[evt].filter((l) => l !== fn) as Listeners[K]
    return this
  }

  close(code = 1000, reason = ''): void {
    this.closedByUser = true
    this.clearReconnectTimer()
    window.removeEventListener('online', this.onOnline)
    this.ws?.close(code, reason)
  }

  connect(): void {
    if (this.closedByUser) return

    if (
      this.ws &&
      (this.ws.readyState === WebSocket.CONNECTING || this.ws.readyState === WebSocket.OPEN)
    ) {
      return
    }

    this.clearReconnectTimer()

    const ws = new WebSocket(this.url, this.protocols)
    this.ws = ws

    ws.onopen = (event) => {
      if (ws !== this.ws) return
      this.isConnected = true

      this.retries = 0
      this.flushPendingMessages()
      this.emit('open', event)
    }

    ws.onerror = (event) => {
      if (ws === this.ws) this.emit('error', event)
    }

    ws.onmessage = (event) => {
      if (ws === this.ws) this.emit('message', event)
    }

    ws.onclose = (event) => {
      if (ws !== this.ws) return

      this.emit('close', event)
      this.isConnected = false

      if (!this.closedByUser) {
        this.scheduleReconnect()
      }
    }
  }

  private flushPendingMessages(): void {
    while (this.pendingMessage.length > 0 && this.isOpen) {
      const msg = this.pendingMessage[0]

      try {
        this._send(msg!)
        this.pendingMessage.shift()
      } catch {
        break
      }
    }
  }

  private onOnline = (): void => {
    if (this.isOpen) return
    this.clearReconnectTimer()
    this.connect()
  }

  private scheduleReconnect = (): void => {
    if (this.closedByUser || this.reconnectTimer != null) return
    const { baseDelay = 1000, maxDelay = 30_000, factor = 2, maxRetries = Infinity } = this.opts
    if (this.retries >= maxRetries) return
    const ceiling = Math.min(maxDelay, baseDelay * factor ** this.retries)
    const delay = Math.random() * ceiling

    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null

      if (this.closedByUser) return

      this.retries++
      this.connect()
    }, delay)
  }

  private _send(msg: BufferSource | Blob | string | object): void {
    if (typeof msg == 'object') {
      const m = JSON.stringify(msg)
      this.ws?.send(m)
      return
    }
    this.ws?.send(msg)
  }
  send(msg: BufferSource | Blob | string | object): void {
    if (!this.isConnected) {
      this.pendingMessage.push(msg)
      return
    }
    this._send(msg)
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer !== null) clearTimeout(this.reconnectTimer)
    this.reconnectTimer = null
  }
}
