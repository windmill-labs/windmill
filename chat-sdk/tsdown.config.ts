import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: ['src/index.ts', 'src/react.ts', 'src/ai-sdk.ts', 'src/assistant-ui.ts'],
  format: ['esm', 'cjs'],
  dts: false,
  external: ['react', 'ai', '@assistant-ui/react'],
  target: 'es2020',
})
