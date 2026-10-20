import { describe, it, expect } from 'vitest'

import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import App from '../App.vue'

describe('App', () => {
  it('mounts renders properly', () => {
    setActivePinia(createPinia())
    const wrapper = mount(App, {
      global: {
        stubs: {
          RouterView: true,
        },
      },
    })
    expect(wrapper.text()).toContain('Skip to main content')
  })
})
