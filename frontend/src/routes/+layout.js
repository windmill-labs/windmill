import { config } from 'zod/v4/core'
import en from 'zod/v4/locales/en.js'

// zod's own config(en()) sits in a module its `sideEffects: false` lets the bundler drop, which
// turns every default message into "Invalid input" in production builds only. zod 4.5 registers
// the locale itself, so this can go on that upgrade. Guarded by assertZodMessagesKept.
config(en())

export const ssr = false
export const prerender = false
