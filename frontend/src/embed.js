import AppPreview from './lib/components/apps/editor/AppPreview.svelte'

globalThis.process = { env: { NODE_ENV: 'production' } }
globalThis.loadApp = (target, value) => {
	new AppPreview({
		target: target,
		props: { app: value }
	})
}
