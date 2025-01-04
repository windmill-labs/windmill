import * as esbuild from 'esbuild-wasm'

let initialized = false

export async function initializeEsbuild() {
	if (!initialized) {
		await esbuild.initialize({
			wasmURL: 'https://unpkg.com/esbuild-wasm/esbuild.wasm', // Use a CDN for simplicity
			worker: true
		})
		initialized = true
	}
}

export { esbuild }
