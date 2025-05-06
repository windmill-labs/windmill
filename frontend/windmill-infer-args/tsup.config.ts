import { defineConfig } from 'tsup'

export default defineConfig({
	entry: ['src/index.ts'],
	format: ['cjs', 'esm'],
	dts: true,
	clean: true,
	treeshake: false,
	outExtension({ format }) {
		return {
			js: format === 'cjs' ? '.js' : '.mjs'
		}
	}
})
