import { fileURLToPath } from 'node:url'
import frontendConfig from '../../../frontend/vite.config.js'

// Harness unit tests that reach into the frontend module graph. They can't run under
// `bun test` (Svelte runes and the SvelteKit aliases both need this build), so they are
// named `*.vitest.ts` — bun's `*.test.ts` sweep skips them and this config claims them.
const FRONTEND_VITE_CONFIG_PATH = fileURLToPath(new URL('../../../frontend/vite.config.js', import.meta.url))
const FRONTEND_TEST_SETUP_PATH = fileURLToPath(
	new URL('../../../frontend/src/lib/test-setup.ts', import.meta.url)
)
const UNIT_TESTS = fileURLToPath(new URL('./**/*.vitest.ts', import.meta.url))

const config = {
	...frontendConfig,
	test: {
		...frontendConfig.test,
		projects: [
			{
				extends: FRONTEND_VITE_CONFIG_PATH,
				test: {
					name: 'server',
					environment: 'node',
					include: [UNIT_TESTS],
					setupFiles: [FRONTEND_TEST_SETUP_PATH]
				}
			}
		]
	}
}

export default config
