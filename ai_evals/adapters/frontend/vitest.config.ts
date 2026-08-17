import { fileURLToPath } from 'node:url'
import frontendConfig from '../../../frontend/vite.config.js'

const FRONTEND_VITE_CONFIG_PATH = fileURLToPath(new URL('../../../frontend/vite.config.js', import.meta.url))
const FRONTEND_TEST_SETUP_PATH = fileURLToPath(
	new URL('../../../frontend/src/lib/test-setup.ts', import.meta.url)
)
const ADAPTER_TEST_PATH = fileURLToPath(new URL('./vitestAdapter.test.ts', import.meta.url))

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
					include: [ADAPTER_TEST_PATH],
					setupFiles: [FRONTEND_TEST_SETUP_PATH]
				}
			}
		]
	}
}

export default config
