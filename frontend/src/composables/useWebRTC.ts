import { ref } from 'vue'

export type DataControlType = string | Blob | ArrayBuffer

export function useWebRTC(
  send: (obj: unknown) => unknown,
  onTrack?: (e: RTCTrackEvent) => void,
  handleControl?: (c: MessageEvent) => void,
) {
  const debug = (...args: unknown[]) => {
    if (import.meta.env.DEV) console.log(...args)
  }

  const remoteVideoRef = ref<HTMLVideoElement | null>(null)
  const peerCon = ref<RTCPeerConnection | null>(null)
  const localTracksVideo = ref<RTCRtpSender | null>(null)
  const localTracksAudio = ref<RTCRtpSender | null>(null)
  const controlChannel = ref<RTCDataChannel | null>(null)
  const pendingMessageControlCh: DataControlType[] = []

  const pendingCandidates: RTCIceCandidateInit[] = []
  const addedTracks = new Set<MediaStreamTrack>()
  let remoteDescriptionSet = false

  const flushPendingCandidates = async () => {
    while (pendingCandidates.length > 0) {
      const candidate = pendingCandidates.shift()
      if (!candidate) continue
      try {
        await peerCon.value?.addIceCandidate(candidate)
      } catch (err) {
        console.warn('addIceCandidate failed', err)
      }
    }
  }

  const init = async () => {
    if (peerCon.value) return

    const pc = new RTCPeerConnection({
      iceServers: [{ urls: 'stun:stun.l.google.com:19302' }],
      rtcpMuxPolicy: 'require',
    })
    peerCon.value = pc

    controlChannel.value ??= pc.createDataChannel('control')
    controlChannel.value.onopen = () => {
      debug('WebRTC connected')
      while (pendingMessageControlCh.length > 0 && controlChannel.value?.readyState === 'open') {
        const msg = pendingMessageControlCh[0]

        try {
          _sendDataControl(msg!)
          pendingMessageControlCh.shift()
        } catch {
          break
        }
      }
    }

    const isIPv6 = (c: RTCIceCandidate) => {
      const addr = c.address ?? c.candidate.split(' ')[4] // fallback parse dari string
      return addr?.includes(':')
    }

    pc.addEventListener('icecandidate', (e) => {
      if (!e.candidate) return
      if (isIPv6(e.candidate)) return
      send({
        type: 'candidate',
        candidate: e.candidate.candidate,
        mid: e.candidate.sdpMid,
        line_index: e.candidate.sdpMLineIndex,
        username_fragment: e.candidate.usernameFragment,
      })
    })

    pc.addEventListener('iceconnectionstatechange', () => {
      console.log('ICE:', pc.iceConnectionState)
    })

    const caps = RTCRtpReceiver.getCapabilities('video')
    // TODO: send capability to server
    console.log('capability video ', caps)

    pc.addEventListener('connectionstatechange', () => debug('pc:', pc.connectionState))
    pc.addEventListener('track', (e) => {
      console.log('new track', e)
      if (remoteVideoRef.value)
        remoteVideoRef.value.srcObject = e.streams[0] ?? new MediaStream([e.track])
      onTrack?.(e)
    })
    pc.addEventListener('datachannel', (e) => {
      const channel = e.channel

      channel.onmessage = (msg) => {
        handleControl!(msg)
      }
    })
  }

  /// attach local media tracks, skipped when the track is already attached
  const addLocalStream = (stream: MediaStream) => {
    const pc = peerCon.value
    if (!pc) return
    for (const track of stream.getTracks()) {
      console.log('track', track)

      if (track.kind == 'video') {
        if (!localTracksVideo.value) localTracksVideo.value = pc.addTrack(track, stream)
        else localTracksVideo.value.replaceTrack(track)
      } else if (track.kind == 'audio') {
        if (!localTracksAudio.value) localTracksAudio.value = pc.addTrack(track, stream)
        else localTracksAudio.value.replaceTrack(track)
      }

      if (addedTracks.has(track)) continue
      addedTracks.add(track)
    }
  }

  /// force the video transceivers to negotiate H264, the recorder on the
  /// backend only understands H264. Chrome/Brave put VP8 first by default
  // TODO: create option to select preferred codecs and reports the current codecs to backend
  const preferH264 = (pc: RTCPeerConnection): void => {
    if (typeof RTCRtpSender.getCapabilities !== 'function') return

    const codecs = RTCRtpSender.getCapabilities('video')?.codecs ?? []
    const h264 = codecs.filter((c) => c.mimeType.toLowerCase() === 'video/h264')
    if (h264.length === 0) {
      console.warn('H264 is not available, recording will not work')
      return
    }
    for (const s of pc.getSenders()) {
      const p = s.getParameters()
      if (p.encodings.length == 0) continue
      p.encodings[0]!.maxBitrate = 2_000_000
      p.encodings[0]!.networkPriority = 'high'
      s.setParameters(p).then()
    }

    for (const transceiver of pc.getTransceivers()) {
      const kind = transceiver.sender.track?.kind ?? transceiver.receiver.track?.kind
      if (kind !== 'video') continue

      try {
        transceiver.setCodecPreferences(h264)
      } catch (err) {
        console.warn('setCodecPreferences failed', err)
      }
    }
  }

  /// create offer + apply it locally, returns the sdp to be sent to the peer
  const createOffer = async (): Promise<string | undefined> => {
    const pc = peerCon.value
    if (!pc) return undefined
    preferH264(pc)
    const offer = await pc.createOffer()
    await pc.setLocalDescription(offer)
    return pc.localDescription?.sdp ?? offer.sdp
  }

  const setRemoteAnswer = async (sdp: string) => {
    const pc = peerCon.value
    if (!pc) return
    await pc.setRemoteDescription({ type: 'answer', sdp })
    remoteDescriptionSet = true
    await flushPendingCandidates()
  }

  const handleRemoteOffer = async (sdp: string): Promise<string | undefined> => {
    const pc = peerCon.value
    if (!pc) return undefined
    if (pc.signalingState !== 'stable') {
      console.warn('handleRemoteOffer while signalingState is', pc.signalingState)
      await pc.setLocalDescription({ type: 'rollback' })
    }
    await pc.setRemoteDescription({ type: 'offer', sdp })
    remoteDescriptionSet = true
    await flushPendingCandidates()
    const answer = await pc.createAnswer()
    await pc.setLocalDescription(answer)
    return pc.localDescription?.sdp ?? answer.sdp
  }

  /// candidates may arrive before the remote description is applied, buffer them
  const addIceCandidate = async (candidate: RTCIceCandidateInit) => {
    if (!remoteDescriptionSet) {
      pendingCandidates.push(candidate)
      return
    }
    try {
      await peerCon.value?.addIceCandidate(candidate)
    } catch (err) {
      console.warn('addIceCandidate failed', err)
    }
  }

  const _sendDataControl = <T extends DataControlType>(msg: T): void => {
    const channel = controlChannel.value!
    if (typeof msg === 'string') channel.send(msg)
    else if (msg instanceof Blob) channel.send(msg)
    else channel.send(msg)
  }

  const sendDataControl = <T extends DataControlType>(msg: T): void => {
    const channel = controlChannel.value
    if (!channel || channel!.readyState !== 'open') {
      pendingMessageControlCh.push(msg)
      return
    }

    _sendDataControl(msg)
  }

  const close = () => {
    peerCon.value?.close()
    peerCon.value = null
    pendingCandidates.length = 0
    addedTracks.clear()
    remoteDescriptionSet = false
  }

  return {
    remoteVideoRef,
    peerCon,
    init,
    addLocalStream,
    createOffer,
    setRemoteAnswer,
    handleRemoteOffer,
    addIceCandidate,
    close,
    localTracksVideo,
    localTracksAudio,
    sendDataControl,
  }
}
