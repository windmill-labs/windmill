import { defineConfig } from 'vite'
import { resolve } from 'path'
import { writeFileSync } from 'fs'
import { exec } from 'child_process'
import { promisify } from 'util'

const execAsync = promisify(exec)
const VERSION = '1.0.12'

export default defineConfig({
	build: {
		lib: {
			entry: resolve(__dirname, '../src/lib/sharedUtils.ts'),
			name: 'sharedUtils',
			fileName: (format) => `lib.${format}.js`,
			formats: ['es']
		},
		outDir: 'dist/sharedUtils',
		rolldownOptions: {
			external: [],
			output: {
				globals: {}
			}
		}
	},
	plugins: [
		{
			name: 'bundle-types',
			async closeBundle() {
				try {
					console.log('Bundling types...')

					// Create a temporary tsconfig for this specific build
					const tempTsConfig = {
						extends: './tsconfig.json',
						compilerOptions: {
							declaration: true,
							emitDeclarationOnly: true,
							outDir: 'dist/sharedUtils',
							skipLibCheck: true,
							noEmit: false
						},
						include: ['src/lib/sharedUtils.ts', 'sharedUtils/sharedUtils.d.ts'],
						exclude: ['node_modules']
					}

					writeFileSync('tsconfig.sharedUtils.json', JSON.stringify(tempTsConfig, null, 2))

					// Use the temporary tsconfig
					await execAsync('npx tsc -p tsconfig.sharedUtils.json')
					console.log('Generating types...')

					// Clean up temp tsconfig
					await execAsync('rm tsconfig.sharedUtils.json')

					// Rename sharedUtils.d.ts to lib.d.ts
					await execAsync('mv dist/sharedUtils/sharedUtils.d.ts dist/sharedUtils/lib.d.ts')

					const pkgJson = {
						name: '@windmill-labs/shared-utils',
						version: VERSION,
						type: 'module',
						private: false,
						main: './lib.es.js',
						module: './lib.es.js',
						exports: {
							'.': './lib.es.js'
						},
						types: './lib.d.ts'
					}

					const jsrJson = {
						name: '@windmill-labs/shared-utils',
						version: VERSION,
						license: 'MIT',
						exports: './lib.es.js'
					}
					writeFileSync(
						resolve(__dirname, '../dist/sharedUtils/package.json'),
						JSON.stringify(pkgJson, null, 2)
					)
					writeFileSync(
						resolve(__dirname, '../dist/sharedUtils/jsr.json'),
						JSON.stringify(jsrJson, null, 2)
					)

					console.log('Types bundled successfully')
				} catch (error) {
					console.error('Error bundling types:', error)
					throw error
				}
			}
		}
	]
})
