<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte'
	import type { InstalledPackage } from './npm_install'

	export let files: Record<string, string>
	export let logs: string = ''

	import NpmInstall from './NpmInstall.svelte'
	import { wmillTs } from './utils'

	export let installed: InstalledPackage[] | undefined = []
	let npm_install: NpmInstall | undefined = undefined

	const dispatch = createEventDispatcher()

	const worker = new Worker(new URL('./worker.ts', import.meta.url), {
		type: 'module'
	})

	onMount(async () => {
		await updatePackageJson()
		build('/index.ts')
	})

	export function onContentChange(activeFile: string) {
		if (activeFile == '/package.json') {
			updatePackageJson()
			build('/index.ts')
		} else {
			build(activeFile)
		}
	}

	const sendMessageToWorker = (type, data) =>
		new Promise((resolve, reject) => {
			const id = Math.random().toString(36).substr(2, 9)
			worker.postMessage({ id, type, data })
			const onMessage = (event) => {
				if (event.data.id === id) {
					worker.removeEventListener('message', onMessage)
					if (event.data.success) {
						resolve(event.data.result)
					} else {
						reject(new Error(event.data.error))
					}
				}
			}
			worker.addEventListener('message', onMessage)
		})

	let initialized = false
	async function build(activeFile: string) {
		if (!initialized) {
			initialized = true
			logs += 'Initializing worker\n'
			await sendMessageToWorker('init', {})
			logs += 'Initialized worker\n'
		}
		console.log('content change', activeFile)
		// for (const {files} of $installed)
		// svelte-lsp error: seems cannot use local variable whose name is the same as imported store
		let input = {}
		Object.keys(files).forEach((path) => {
			input[path] = files[path]
		})
		input['/wmill.ts'] = wmillTs

		for (const pkg of installed ?? []) {
			const base = `node_modules/${pkg.name}/`
			for (const { path, content } of pkg.files) {
				const resolved = base + path
				input[resolved] = content
			}
		}

		logs += '\n'
		try {
			let started = Date.now()
			logs += 'Build started...\n'
			const res = await sendMessageToWorker('build', { files: input })
			dispatch('build', res)
			logs += 'Build successful in ' + (Date.now() - started) + 'ms'
		} catch (e) {
			dispatch('buildFailed')
			logs += e.message
		}
		logs += '\n'
	}

	async function updatePackageJson() {
		installed = await npm_install?.parsePackageJson(files?.['/package.json'], true)
		console.log('install done', installed)
		dispatch('install', installed)
		// const reactPkgJson = fs.read(cwd + '/package-lock.json', 'utf-8')

		// console.log({ reactPkgJson })
	}
</script>

<NpmInstall bind:logs bind:this={npm_install} />
