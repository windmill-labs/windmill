<script lang="ts">
	import type { AIProvider } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Button from '../common/button/Button.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { AI_PROVIDERS, testKey } from './lib'
	import { untrack } from 'svelte'

	interface Props {
		disabled?: boolean
		apiKey?: string | undefined
		workspace?: string | undefined
		resourcePath?: string | undefined
		/** An unsaved resource value to test as is, instead of a stored resource. */
		resourceValue?: Record<string, any> | undefined
		aiProvider: AIProvider
		model?: string | undefined
	}

	let {
		disabled = false,
		apiKey = undefined,
		workspace = undefined,
		resourcePath = undefined,
		resourceValue = undefined,
		aiProvider,
		model = undefined
	}: Props = $props()

	let loading = $state(false)
	// For an unsaved resource, or without a model from the caller, the model is only a
	// guess: deployment-named providers (Azure, Bedrock, custom endpoints) reject a model
	// the account lacks, which would read as an invalid key, so the guess stays editable.
	let typedModel = $state(
		untrack(() => model ?? AI_PROVIDERS[aiProvider]?.defaultModels[0] ?? '')
	)
	let modelEditable = $derived(!!resourceValue || !model)
	let testedModel = $derived(modelEditable ? typedModel.trim() : model)
</script>

<div class="flex flex-row items-center gap-1">
	{#if modelEditable}
		<TextInput
			bind:value={typedModel}
			size="md"
			inputProps={{ placeholder: 'Model to test', 'aria-label': 'Model to test' }}
		/>
	{/if}
	<Button
		unifiedSize="md"
		variant="default"
		disabled={disabled || !testedModel}
		{loading}
		onClick={async () => {
			loading = true
			try {
				const abortController = new AbortController()
				setTimeout(() => {
					abortController.abort()
				}, 10000)

				await testKey({
					apiKey,
					workspace,
					resourcePath,
					resourceValue,
					messages: [
						{
							role: 'user',
							content: "this is a test, simply reply with 'ok'"
						}
					],
					abortController,
					aiProvider,
					model: testedModel
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
</div>
