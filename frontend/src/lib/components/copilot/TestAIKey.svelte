<script lang="ts">
	import type { AIProvider } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Button from '../common/button/Button.svelte'
	import { testKey } from './lib'
	export let disabled = false
	export let apiKey: string | undefined = undefined
	export let resourcePath: string | undefined = undefined
	export let aiProvider: AIProvider
	export let model: string
	let loading = false
</script>

<Button
	size="xs"
	variant="contained"
	color="dark"
	{disabled}
	{loading}
	on:click={async () => {
		loading = true
		try {
			const abortController = new AbortController()
			setTimeout(() => {
				abortController.abort()
			}, 10000)

			await testKey({
				apiKey,
				resourcePath,
				messages: [
					{
						role: 'user',
						content: "this is a test, simply reply with 'ok'"
					}
				],
				abortController,
				aiProvider,
				model
			})
			sendUserToast('Valid key')
		} catch (err) {
			if (err.message === 'Request was aborted.') {
				sendUserToast('Could not validate key within 10s', true)
			} else {
				sendUserToast(`Invalid key: ${err}`, true)
			}
		} finally {
			loading = false
		}
	}}
>
	{#if apiKey}
		Test key
	{:else}
		Test
	{/if}
</Button>
