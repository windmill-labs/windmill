import { defineConfig } from 'vite'
import { resolve } from 'path'
import { copyFileSync, readFileSync, writeFileSync } from 'fs'
import dts from 'vite-plugin-dts'

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
		dts({
			include: ['src/lib/components/apps/editor/appPolicy.ts'],
			outDir: 'dist/sharedUtils',
			rollupTypes: false,
			// Generate individual .d.ts files
			insertTypesEntry: false,
			// Skip diagnostics to avoid TypeScript errors from other files
			skipDiagnostics: true,
			tsconfigPath: './tsconfig.json',
			entryRoot: 'src/lib/components/apps/editor'
		}),
		{
			name: 'rename-and-fix-types',
			closeBundle() {
				const dtsPath = resolve(__dirname, 'dist/sharedUtils/appPolicy.d.ts')
				const targetPath = resolve(__dirname, 'dist/sharedUtils/lib.d.ts')

				// Read the generated .d.ts file
				const content = readFileSync(dtsPath, 'utf-8')

				// Write to lib.d.ts (main entry point for types)
				writeFileSync(targetPath, content)
			}
		},
		{
			name: 'copy-package-json',
			closeBundle() {
				// Copy package.json to dist
				copyFileSync(
					resolve(__dirname, 'package.sharedUtils.json'),
					resolve(__dirname, 'dist/sharedUtils/package.json')
				)
			}
		}
	]
})
