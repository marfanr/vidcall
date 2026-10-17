<script setup lang="ts">
import { ref } from 'vue';

const navigationItems = ref([
  { name: 'Meet', icon: 'video', active: true },
  { name: 'Contacts', icon: 'users', active: false },
]);

const showProfileMenu = ref(false);

const upcomingMeetings = ref([
  { id: 1, title: 'Team Standup', time: '09:00', date: 'Today', participants: 8 },
  { id: 2, title: 'Project Review', time: '14:00', date: 'Today', participants: 12 },
  { id: 3, title: 'Client Call', time: '16:30', date: 'Tomorrow', participants: 5 },
]);

const recentMeetings = ref([
  { id: 4, title: 'Sprint Planning', time: '10:00', date: 'Yesterday', participants: 10 },
  { id: 5, title: 'Design Review', time: '15:00', date: '2 days ago', participants: 6 },
]);

const setActive = (name: string) => {
  navigationItems.value = navigationItems.value.map(item => ({
    ...item,
    active: item.name === name
  }));
};

const toggleProfileMenu = () => {
  showProfileMenu.value = !showProfileMenu.value;
};

const closeProfileMenu = () => {
  showProfileMenu.value = false;
};
</script>

<template>
  <div class="min-h-screen bg-gray-100" @click="closeProfileMenu">
    <header class="bg-white border-b border-gray-200 px-6 py-3">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-8">

          <nav class="flex items-center gap-1">
            <button v-for="item in navigationItems" :key="item.name" @click="setActive(item.name)" :class="[
              'px-4 py-2 rounded-lg text-sm font-medium transition-all',
              item.active
                ? 'bg-[#1d4cff]/10 text-[#1d4cff]'
                : 'text-gray-600 hover:bg-gray-100'
            ]">
              {{ item.name }}
            </button>
          </nav>
        </div>

        <!-- Right: User -->
        <div class="flex items-center gap-4">
          <button class="p-2 hover:bg-gray-100 rounded-full">
            <span>🔔</span>
          </button>

          <!-- Profile Dropdown -->
          <div class="relative" @click.stop>
            <button @click="toggleProfileMenu"
              class="w-8 h-8 bg-[#1d4cff] rounded-full flex items-center justify-center hover:opacity-90 transition-opacity">
              <span class="text-white text-sm font-medium">A</span>
            </button>

            <!-- Dropdown Menu -->
            <div v-if="showProfileMenu"
              class="absolute right-0 mt-2 w-48 bg-white rounded-lg shadow-lg border border-gray-200 py-1 z-50">
              <button
                class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 flex items-center gap-2">
                <span>👤</span> Profile
              </button>
              <button
                class="w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100 flex items-center gap-2">
                <span>⚙️</span> Settings
              </button>
              <hr class="my-1 border-gray-200">
              <button class="w-full px-4 py-2 text-left text-sm text-red-600 hover:bg-gray-100 flex items-center gap-2">
                <span>🚪</span> Logout
              </button>
            </div>
          </div>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="p-8">
      <!-- Header -->
      <header class="mb-8">
        <h1 class="text-2xl font-normal text-gray-800">Upcoming meetings</h1>
      </header>

      <!-- Meetings Grid -->
      <div class="grid grid-cols-3 gap-4 mb-8">
        <div v-for="meeting in upcomingMeetings" :key="meeting.id"
          class="bg-white rounded-lg p-4 shadow-sm border border-gray-200 hover:shadow-md transition-shadow cursor-pointer">
          <p class="text-sm text-gray-500 mb-1">{{ meeting.date }}</p>
          <h3 class="font-medium text-gray-800 mb-2">{{ meeting.title }}</h3>
          <p class="text-sm text-gray-500">{{ meeting.time }}</p>
        </div>
      </div>

      <!-- Recent Section -->
      <div>
        <h2 class="text-lg font-normal text-gray-800 mb-4">Recent</h2>
        <div class="space-y-2">
          <div v-for="meeting in recentMeetings" :key="meeting.id"
            class="flex items-center gap-4 p-3 bg-white rounded-lg border border-gray-200 hover:bg-gray-50 cursor-pointer">
            <div class="w-10 h-10 bg-gray-200 rounded-full flex items-center justify-center">
              <span class="text-sm">📹</span>
            </div>
            <div class="flex-1">
              <h4 class="font-medium text-gray-800">{{ meeting.title }}</h4>
              <p class="text-sm text-gray-500">{{ meeting.date }} • {{ meeting.participants }} participants</p>
            </div>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>
