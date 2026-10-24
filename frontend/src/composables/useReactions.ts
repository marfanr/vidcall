import { ref } from 'vue'
import type { FloatingReaction } from '@/types/video-call'

export function useReactions() {
  const floating = ref<FloatingReaction[]>([])
  let reactionSeq = 0

  const showReaction = (emoji: string) => {
    const key = ++reactionSeq
    floating.value.push({ key, emoji, left: 10 + Math.random() * 80 })
    setTimeout(() => {
      floating.value = floating.value.filter((r) => r.key !== key)
    }, 2500)
  }

  return { floating, showReaction }
}
