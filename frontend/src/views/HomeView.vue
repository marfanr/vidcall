<script setup lang="ts">
import { api } from '@/lib/axios'
import { Bell, UsersRoundIcon } from 'lucide-vue-next'
import { nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { nanoid } from 'nanoid'
import { createFocusTrap, type FocusTrap } from 'focus-trap'
import { useAuthStore } from '@/stores/auth'

/* ---------- navigation ---------- */
const navigationItems = ref([
  { name: 'Meet', icon: 'video', active: true },
  { name: 'Contacts', icon: 'users', active: false },
])

const setActive = (name: string) => {
  navigationItems.value = navigationItems.value.map((item) => ({
    ...item,
    active: item.name === name,
  }))
}

/* ---------- profile menu ---------- */
const showProfileMenu = ref(false)
const toggleProfileMenu = () => (showProfileMenu.value = !showProfileMenu.value)
const closeProfileMenu = () => (showProfileMenu.value = false)

const upcomingMeetings = ref([
  { id: 1, title: 'Team Standup', time: '09:00', date: 'Today', participants: 8 },
  { id: 2, title: 'Project Review', time: '14:00', date: 'Today', participants: 12 },
  { id: 3, title: 'Client Call', time: '16:30', date: 'Tomorrow', participants: 5 },
])

const recentMeetings = ref([
  { id: 4, title: 'Sprint Planning', time: '10:00', date: 'Yesterday', participants: 10 },
  { id: 5, title: 'Design Review', time: '15:00', date: '2 days ago', participants: 6 },
])

type Mode = 'now' | 'scheduled'

const modes: { key: Mode; label: string }[] = [
  { key: 'now', label: 'Now' },
  { key: 'scheduled', label: 'Scheduled' },
]

const today = () => new Date().toLocaleDateString('en-CA') // yyyy-mm-dd, local time
const genPasscode = () => Math.random().toString(36).slice(2, 8)
const personalId = nanoid()

const showNewMeeting = ref(false)
const mode = ref<Mode>('now')

const nowForm = reactive({
  topic: "Arfan's meeting",
  meetingId: 'auto' as 'auto' | 'personal' | 'custom',
  hostVideo: true,
  participantVideo: false,
  muteOnEntry: true,
  waitingRoom: true,
  requestToJoin: true,
})

const nowToggles = [
  { key: 'hostVideo', label: 'Host video on' },
  { key: 'participantVideo', label: 'Participant video on' },
  { key: 'muteOnEntry', label: 'Mute participants on entry' },
  { key: 'waitingRoom', label: 'Waiting room' },
  { key: 'requestToJoin', label: 'request to join' },
] as const

const scheduledForm = reactive({
  topic: '',
  date: today(),
  time: '09:00',
  duration: '45 minutes',
  timezone: 'WIB (UTC+7)',
  passcode: genPasscode(),
  waitingRoom: true,
  requestToJoin: true,
})

const durations = ['15 minutes', '30 minutes', '45 minutes', '1 hour', '2 hours']
const timezones = ['WIB (UTC+7)', 'WITA (UTC+8)', 'WIT (UTC+9)']

const submitting = ref(false)
const submitError = ref('')
const submitSuccess = ref('')

const openNewMeeting = () => {
  mode.value = 'now'
  scheduledForm.date = today()
  scheduledForm.passcode = genPasscode()
  submitError.value = ''
  submitSuccess.value = ''
  showNewMeeting.value = true
}

const closeNewMeeting = () => (showNewMeeting.value = false)

const currentUserId = (): string => {
  const token = useAuthStore().accessToken
  if (!token) return ''
  try {
    const payload = JSON.parse(atob(token.split('.')[1] ?? ''))
    return String(payload.sub ?? '')
  } catch {
    return ''
  }
}

const submitMeeting = async () => {
  if (submitting.value) return
  submitError.value = ''
  submitSuccess.value = ''
  submitting.value = true

  try {
    if (mode.value === 'now') {
      const options = JSON.stringify({
        host_video: nowForm.hostVideo,
        participant_video: nowForm.participantVideo,
        mute_on_entry: nowForm.muteOnEntry,
        waiting_room: nowForm.waitingRoom,
      })

      await api.post('/room', {
        alias: nowForm.topic.trim() || 'Untitled meeting',
        owner_id: currentUserId(),
        passcode: genPasscode(),
        options,
      })

      submitSuccess.value = 'Meeting started. Sharing your room link…'
      closeNewMeeting()
    } else {
      const f = scheduledForm
      upcomingMeetings.value.push({
        id: Date.now(),
        title: f.topic.trim() || 'Untitled meeting',
        time: f.time,
        date: f.date === today() ? 'Today' : f.date,
        participants: 1,
      })
      submitSuccess.value = 'Meeting scheduled.'
      closeNewMeeting()
    }
  } catch (err) {
    const axiosErr = err as { response?: { data?: { error?: string } } }
    submitError.value =
      axiosErr.response?.data?.error || 'Could not create the meeting. Please try again.'
  } finally {
    submitting.value = false
  }
}

/* ---------- modal focus trap ---------- */
const dialogEl = ref<HTMLElement | null>(null)
let trap: FocusTrap | null = null

watch(showNewMeeting, async (open) => {
  if (open) {
    await nextTick()
    if (dialogEl.value) {
      trap = createFocusTrap(dialogEl.value, { returnFocusOnDeactivate: true })
      trap.activate()
    }
  } else {
    trap?.deactivate()
    trap = null
  }
})

const onKeydown = (e: KeyboardEvent) => {
  if (e.key !== 'Escape') return
  if (showNewMeeting.value) closeNewMeeting()
  else if (showProfileMenu.value) closeProfileMenu()
}
onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
  trap?.deactivate()
})

/* shared classes */
const inputCls =
  'rounded-md border border-gray-300 px-3 py-2.5 outline-none transition focus:border-primary focus:ring-3 focus:ring-primary/15'
const radioCls =
  'flex cursor-pointer items-center gap-x-3 rounded-md border border-gray-300 px-3 py-2.5 has-checked:border-primary has-checked:bg-primary/5'
</script>

<template>
  <div class="min-h-screen bg-gray-100" @click="closeProfileMenu">
    <header class="bg-white border-b border-gray-200 px-6 py-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-8">
          <nav class="flex items-center gap-1" aria-label="Primary">
            <button
              v-for="item in navigationItems"
              :key="item.name"
              type="button"
              @click="setActive(item.name)"
              :aria-current="item.active ? 'page' : undefined"
              :class="[
                'px-4 py-2 rounded-lg text-sm font-medium transition-all',
                item.active
                  ? 'bg-primary/10 text-primary'
                  : 'text-gray-600 hover:bg-gray-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
              ]"
            >
              {{ item.name }}
            </button>
          </nav>
        </div>

        <div class="flex items-center gap-4">
          <button
            type="button"
            aria-label="Notifications"
            class="p-2 hover:bg-gray-100 rounded-full focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          >
            <Bell :size="20" aria-hidden="true" />
          </button>

          <!-- Profile Dropdown -->
          <div class="relative" @click.stop>
            <button
              type="button"
              @click="toggleProfileMenu"
              aria-haspopup="true"
              :aria-expanded="showProfileMenu"
              aria-label="Account menu"
              class="w-8 h-8 bg-primary rounded-full flex items-center justify-center hover:opacity-90 transition-opacity focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
            >
              <span class="text-white text-sm font-medium" aria-hidden="true">A</span>
            </button>

            <div
              v-if="showProfileMenu"
              class="absolute right-0 mt-2 w-48 bg-white rounded-lg shadow-lg border border-gray-200 py-1 z-50"
            >
              <button
                type="button"
                class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 flex items-center gap-2"
              >
                Profile
              </button>
              <button
                type="button"
                class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 flex items-center gap-2"
              >
                Settings
              </button>
              <hr class="my-1 border-gray-200" />
              <button
                type="button"
                class="w-full px-4 py-2 text-left text-sm text-red-600 hover:bg-gray-100 flex items-center gap-2"
              >
                Logout
              </button>
            </div>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main id="main" class="p-8 relative">
      <header class="mb-8">
        <h1 class="text-2xl font-normal text-gray-800">Upcoming meetings</h1>
      </header>

      <p
        v-if="submitSuccess"
        role="status"
        class="mb-6 rounded-md border border-green-200 bg-green-50 px-4 py-3 text-sm text-green-800"
      >
        {{ submitSuccess }}
      </p>

      <!-- Meetings Grid -->
      <div class="grid grid-cols-6 gap-4 mb-8">
        <button
          type="button"
          @click.stop="openNewMeeting"
          class="bg-white rounded-lg p-4 shadow-sm border border-gray-200 hover:shadow-md cursor-pointer flex flex-col justify-center items-center gap-y-4 text-primary hover:bg-primary hover:text-white transition-all duration-100 hover:border-transparent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        >
          <UsersRoundIcon :size="30" aria-hidden="true" />
          <span class="text-xs">New Meeting</span>
        </button>

        <button
          v-for="meeting in upcomingMeetings"
          :key="meeting.id"
          type="button"
          class="bg-white rounded-lg p-4 shadow-sm border border-gray-200 hover:shadow-md transition-shadow cursor-pointer text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        >
          <p class="text-sm text-gray-600 mb-1">{{ meeting.date }}</p>
          <span class="block font-medium text-gray-800 mb-2">{{ meeting.title }}</span>
          <p class="text-sm text-gray-600">{{ meeting.time }}</p>
        </button>
      </div>

      <!-- Recent Section -->
      <div>
        <h2 class="text-lg font-normal text-gray-800 mb-4">Recent</h2>
        <div class="space-y-2">
          <button
            v-for="meeting in recentMeetings"
            :key="meeting.id"
            type="button"
            class="w-full flex items-center gap-4 p-3 bg-white rounded-lg border border-gray-200 hover:bg-gray-50 cursor-pointer text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          >
            <span
              class="w-10 h-10 bg-gray-200 rounded-full flex items-center justify-center"
              aria-hidden="true"
            >
              <span class="text-sm">📹</span>
            </span>
            <span class="flex-1">
              <span class="block font-medium text-gray-800">{{ meeting.title }}</span>
              <span class="block text-sm text-gray-600">
                {{ meeting.date }} • {{ meeting.participants }} participants
              </span>
            </span>
          </button>
        </div>
      </div>
    </main>

    <!-- New Meeting modal -->
    <Teleport to="body">
      <Transition
        enter-active-class="transition duration-150 ease-out"
        enter-from-class="opacity-0"
        leave-active-class="transition duration-100 ease-in"
        leave-to-class="opacity-0"
      >
        <div
          v-if="showNewMeeting"
          class="fixed inset-0 z-99 flex items-start justify-center bg-black/40 backdrop-blur-[2px] px-4"
          @click.self="closeNewMeeting"
        >
          <div
            ref="dialogEl"
            role="dialog"
            aria-modal="true"
            aria-labelledby="nm-title"
            class="mt-10 flex max-h-[calc(100vh-5rem)] w-150 max-w-full flex-col rounded-lg border border-gray-200 bg-white shadow-2xl"
          >
            <!-- header -->
            <div class="flex items-center justify-between px-6 pt-6 pb-4">
              <h3 id="nm-title" class="text-xl font-medium text-gray-900">New Meeting</h3>
              <button
                type="button"
                aria-label="Close"
                @click="closeNewMeeting"
                class="grid size-8 place-items-center rounded-full text-gray-500 transition hover:bg-gray-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-primary"
              >
                <svg
                  class="size-5"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  aria-hidden="true"
                >
                  <path d="M6 6l12 12M18 6L6 18" />
                </svg>
              </button>
            </div>

            <!-- tabs -->
            <div role="tablist" class="flex gap-x-2 px-6 text-sm font-light">
              <button
                v-for="m in modes"
                :key="m.key"
                role="tab"
                type="button"
                :aria-selected="mode === m.key"
                @click="mode = m.key"
                class="w-full cursor-pointer rounded-md border border-gray-300 p-4 text-center transition hover:border-primary aria-selected:border-primary aria-selected:bg-primary aria-selected:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              >
                {{ m.label }}
              </button>
            </div>

            <!-- body -->
            <div class="flex-1 overflow-y-auto px-6 py-5 text-sm">
              <!-- NOW -->
              <form
                v-if="mode === 'now'"
                class="flex flex-col gap-y-5"
                @submit.prevent="submitMeeting"
              >
                <div class="flex flex-col gap-y-1.5">
                  <label for="now-topic" class="text-gray-600">Topic</label>
                  <input id="now-topic" v-model="nowForm.topic" type="text" :class="inputCls" />
                </div>

                <fieldset class="flex flex-col gap-y-2">
                  <legend class="mb-1 text-gray-600">Meeting ID</legend>
                  <label :class="radioCls">
                    <input
                      v-model="nowForm.meetingId"
                      type="radio"
                      value="auto"
                      class="accent-primary"
                    />
                    <span>Generate automatically</span>
                  </label>
                  <label :class="radioCls">
                    <input
                      v-model="nowForm.meetingId"
                      type="radio"
                      value="personal"
                      class="accent-primary"
                    />
                    <span>Personal meeting ID</span>
                    <span class="ml-auto font-mono text-gray-500">{{ personalId }}</span>
                  </label>
                  <label :class="radioCls">
                    <input
                      v-model="nowForm.meetingId"
                      type="radio"
                      value="custom"
                      class="accent-primary"
                    />
                    <span>Custom</span>
                  </label>
                </fieldset>

                <div
                  class="flex flex-col divide-y divide-gray-100 rounded-md border border-gray-200"
                >
                  <label
                    v-for="t in nowToggles"
                    :key="t.key"
                    class="flex cursor-pointer items-center justify-between px-3 py-3"
                  >
                    <span>{{ t.label }}</span>
                    <span class="relative inline-flex">
                      <input v-model="nowForm[t.key]" type="checkbox" class="peer sr-only" />
                      <span
                        class="h-5 w-9 rounded-full bg-gray-300 transition peer-checked:bg-primary peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-primary"
                      ></span>
                      <span
                        class="absolute top-0.5 left-0.5 size-4 rounded-full bg-white shadow transition peer-checked:translate-x-4"
                      ></span>
                    </span>
                  </label>
                </div>
              </form>

              <!-- SCHEDULED -->
              <form v-else class="flex flex-col gap-y-5" @submit.prevent="submitMeeting">
                <div class="flex flex-col gap-y-1.5">
                  <label for="sc-topic" class="text-gray-600">Topic</label>
                  <input
                    id="sc-topic"
                    v-model="scheduledForm.topic"
                    type="text"
                    placeholder="Weekly sync"
                    :class="inputCls"
                  />
                </div>

                <div class="grid grid-cols-2 gap-x-3">
                  <div class="flex flex-col gap-y-1.5">
                    <label for="sc-date" class="text-gray-600">Date</label>
                    <input
                      id="sc-date"
                      v-model="scheduledForm.date"
                      type="date"
                      :min="today()"
                      :class="inputCls"
                    />
                  </div>
                  <div class="flex flex-col gap-y-1.5">
                    <label for="sc-time" class="text-gray-600">Start time</label>
                    <input
                      id="sc-time"
                      v-model="scheduledForm.time"
                      type="time"
                      :class="inputCls"
                    />
                  </div>
                </div>

                <div class="grid grid-cols-2 gap-x-3">
                  <div class="flex flex-col gap-y-1.5">
                    <label for="sc-dur" class="text-gray-600">Duration</label>
                    <select
                      id="sc-dur"
                      v-model="scheduledForm.duration"
                      :class="[inputCls, 'bg-white']"
                    >
                      <option v-for="d in durations" :key="d" :value="d">{{ d }}</option>
                    </select>
                  </div>
                  <div class="flex flex-col gap-y-1.5">
                    <label for="sc-tz" class="text-gray-600">Time zone</label>
                    <select
                      id="sc-tz"
                      v-model="scheduledForm.timezone"
                      :class="[inputCls, 'bg-white']"
                    >
                      <option v-for="z in timezones" :key="z" :value="z">{{ z }}</option>
                    </select>
                  </div>
                </div>

                <div class="flex flex-col gap-y-1.5">
                  <label for="sc-pass" class="text-gray-600">Passcode</label>
                  <input
                    id="sc-pass"
                    v-model="scheduledForm.passcode"
                    type="text"
                    :class="[inputCls, 'font-mono']"
                  />
                </div>

                <label
                  class="flex cursor-pointer items-center justify-between rounded-md border border-gray-200 px-3 py-3"
                >
                  <span>Waiting room</span>
                  <span class="relative inline-flex">
                    <input
                      v-model="scheduledForm.waitingRoom"
                      type="checkbox"
                      class="peer sr-only"
                    />
                    <span
                      class="h-5 w-9 rounded-full bg-gray-300 transition peer-checked:bg-primary peer-focus-visible:outline-2 peer-focus-visible:outline-offset-2 peer-focus-visible:outline-primary"
                    ></span>
                    <span
                      class="absolute top-0.5 left-0.5 size-4 rounded-full bg-white shadow transition peer-checked:translate-x-4"
                    ></span>
                  </span>
                </label>
              </form>
            </div>

            <!-- footer -->
            <div class="flex items-center justify-end gap-x-2 border-t border-gray-100 px-6 py-4">
              <p v-if="submitError" role="alert" class="mr-auto text-sm text-red-600">
                {{ submitError }}
              </p>
              <button
                type="button"
                @click="closeNewMeeting"
                class="rounded-md border border-gray-300 px-5 py-2.5 text-sm transition hover:bg-gray-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              >
                Cancel
              </button>
              <button
                type="button"
                @click="submitMeeting"
                :disabled="submitting"
                :aria-busy="submitting"
                class="rounded-md bg-primary px-5 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover disabled:opacity-60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              >
                {{ submitting ? 'Working…' : mode === 'now' ? 'Start meeting' : 'Schedule' }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
