<script lang="ts">
	import type { AIProvider } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Button from '../common/button/Button.svelte'
	import { testKey } from './lib'

	interface Props {
		disabled?: boolean
		apiKey?: string | undefined
		resourcePath?: string | undefined
		aiProvider: AIProvider
		model: string
	}

	let {
		disabled = false,
		apiKey = undefined,
		resourcePath = undefined,
		aiProvider,
		model
	}: Props = $props()

	let loading = $state(false)
</script>

<Button
	unifiedSize="md"
	variant="default"
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
	>Test key
</Button>
