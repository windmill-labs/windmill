import { defineConfig } from 'cypress'

export default defineConfig({
  env: {
    baseUrl: 'http://localhost:8000',
  },
  e2e: {
    setupNodeEvents(on, config) {},
  },
})
