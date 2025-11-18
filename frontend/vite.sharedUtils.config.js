import { defineConfig } from 'vite'
import { resolve } from 'path'
import { copyFileSync } from 'fs'

export default defineConfig({
	build: {
		lib: {
			entry: resolve(__dirname, 'src/lib/components/apps/editor/appPolicy.ts'),
			name: 'sharedUtils',
			fileName: (format) => `lib.${format}.js`,
			formats: ['es']
		},
		outDir: 'dist/sharedUtils',
		rollupOptions: {
			// Externalize dependencies you don't want bundled
			external: [],
			output: {
				globals: {}
			}
		}
	},
	plugins: [
		{
			name: 'copy-package-json',
			closeBundle() {
				// Option 1: Copy an existing package.json
				copyFileSync(
					resolve(__dirname, 'package.sharedUtils.json'),
					resolve(__dirname, 'dist/sharedUtils/package.json')
				)
			}
		}
	]
})
