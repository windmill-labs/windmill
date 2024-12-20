<script lang="ts">
	import ResultJobLoader from '$lib/components/ResultJobLoader.svelte'
	import { onMount } from 'svelte'
	import type { Runnable } from '../../inputType'
	import type { CancelablePromise, InlineScript } from '../../types'

	export let id: string
	export let runnable: Runnable

	export let cb: ((inlineScript?: InlineScript) => CancelablePromise<void>) | undefined = undefined

	let resultJobLoader: ResultJobLoader
	onMount(() => {
		cb = (inlineScript?: InlineScript, setRunnableJobEditorPanel?: boolean) => {
			let rejectCb: (err: Error) => void
			let p: Partial<CancelablePromise<any>> = new Promise<any>((resolve, reject) => {
				rejectCb = reject
				// executeComponent(true, inlineScript, setRunnableJobEditorPanel, undefined, {
				// 	done: (x) => {
				// 		resolve(x)
				// 	},
				// 	cancel: () => {
				// 		reject()
				// 	},
				// 	error: (e) => {
				// 		console.error(e)
				// 		reject(e)
				// 	}
				// }).catch(reject)
			})
			p.cancel = () => {
				resultJobLoader?.cancelJob()
				rejectCb(new Error('Canceled'))
			}

			return p as CancelablePromise<void>
		}
	})
</script>

<ResultJobLoader bind:this={resultJobLoader} />
