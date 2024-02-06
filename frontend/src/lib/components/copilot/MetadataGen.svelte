<script lang="ts">
	import { getCompletion } from './lib'
	import type { NewScript } from '$lib/gen'
	import { isInitialCode } from '$lib/script_helpers'
	import { Loader2 } from 'lucide-svelte'
	import type { ChatCompletionMessageParam } from 'openai/resources'
	import { copilotInfo, metadataCompletionEnabled } from '$lib/stores'
	import Label from '../Label.svelte'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { sendUserToast } from '$lib/toast'

	type Config = {
		system: string
		user: string
		mode: 'automatic' | 'manual'
	}

	const configs: {
		summary: Config
		description: Config
	} = {
		summary: {
			system: `
You are a helpful AI assistant. You generate very brief summaries from scripts.
The summaries need to be as short as possible (maximum 8 words) and only give a global idea. Do not specify the programming language. Do not use any punctation. Avoid using prepositions and articles.
Examples: List the commits of a GitHub repository, Divide a number by 16, etc..
`,
			user: `
		Generate a very short summary for the script below:
\'\'\'{lang}
{code}
\`\`\`
`,
			mode: 'automatic'
		},
		description: {
			system: `
You are a helpful AI assistant. You generate descriptions from scripts.
These descriptions are used to explain to other users what the script does, on particular on the input and what it returns.
Descriptions should contain a maximum of 4-5 sentences.
The description should focus on what it does and should not contain what concepts it uses (e.g. function named main, export an async function, etc...)
`,
			user: `
		Generate a description for the script below:
\'\'\'{lang}
{code}
\`\`\`
`,
			mode: 'manual'
		}
	}

	export let content: string | undefined
	export let code: string
	export let lang: NewScript.language
	export let configName: keyof typeof configs

	export let el: HTMLElement | undefined = undefined
	export let label: string

	let generatedContent = ''
	let genEl: HTMLElement | undefined = undefined
	let active = false
	let loading = false
	let abortController = new AbortController()
	let manualDisabled = false

	export let focused = false
	const updateFocus = (val) => {
		focused = val
	}

	let config: Config = configs[configName]

	async function generateContent(automatic = false) {
		abortController = new AbortController()
		loading = true
		try {
			const messages: ChatCompletionMessageParam[] = [
				{
					role: 'system',
					content: config.system
				},
				{
					role: 'user',
					content: config.user.replace('{lang}', lang).replace('{code}', code)
				}
			]
			const response = await getCompletion(messages, abortController)
			generatedContent = ''
			for await (const chunk of response) {
				const toks = chunk.choices[0]?.delta?.content || ''
				generatedContent += toks
				if (el !== undefined && genEl !== undefined) {
					el.style.height = Math.max(genEl.scrollHeight + 34, 58) + 'px'
				}
			}
		} catch (err) {
			if (!abortController.signal.aborted) {
				if (automatic) {
					console.error('Could not generate summary ' + err)
				} else {
					sendUserToast('Could not generate summary: ' + err, true)
				}
			}
		} finally {
			loading = false
		}
	}

	if (
		$copilotInfo.exists_openai_resource_path &&
		$metadataCompletionEnabled &&
		config.mode === 'automatic' &&
		code &&
		!content &&
		!isInitialCode(code)
	) {
		generateContent(true)
	}

	const dispatch = createEventDispatcher()

	$: if (content) {
		dispatch('change', { content })
	}

	$: active =
		$copilotInfo.exists_openai_resource_path &&
		$metadataCompletionEnabled &&
		!content &&
		(loading || focused || !!generatedContent) &&
		!manualDisabled

	$: focused && (manualDisabled = false)

	onDestroy(() => {
		abortController.abort()
	})
</script>

<div class="relative">
	<div class="flex flex-row" />
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<Label {label}>
		<div
			class="relative"
			on:keydown={(event) => {
				if (!$copilotInfo.exists_openai_resource_path || !$metadataCompletionEnabled) {
					return
				}
				if (event.key === 'Tab') {
					if (!loading && generatedContent) {
						event.preventDefault()
						content = generatedContent
						generatedContent = ''
					} else if (!loading && !content) {
						event.preventDefault()
						generateContent()
					}
				} else if (event.key === 'Escape') {
					event.preventDefault()
					event.stopPropagation()
					if (loading) {
						abortController.abort()
					} else {
						manualDisabled = true
					}
				}
			}}
		>
			<div
				class={'absolute left-0.5  flex flex-row pl-1 gap-2 items-start top-[0.3rem] pointer-events-none'}
			>
				{#if active}
					<span
						class="absolute text-xs text-sky-900 bg-sky-100 dark:text-sky-200 dark:bg-gray-700 p-1 rounded-md flex flex-row items-center justify-center gap-2 w-32 shrink-0"
					>
						{#if loading}
							<Loader2 class="animate-spin text-gray-500 dark:text-gray-400" size={16} />
						{/if}
						<div>
							<span class="px-1 py-0.5 rounded-md text-2xs text-bold bg-white dark:bg-surface">
								{#if loading}
									ESC
								{:else}
									TAB
								{/if}
							</span>
							{#if loading}
								to cancel
							{:else if generatedContent}
								to accept
							{:else}
								to generate
							{/if}
						</div>
					</span>
					<span
						class="text-sm leading-6 indent-[8.5rem] text-gray-500 dark:text-gray-400 pr-1"
						bind:this={genEl}
					>
						{generatedContent}
					</span>
				{/if}
			</div>
			<slot {updateFocus} {active} classNames={active && !generatedContent ? '!pl-[8.7rem]' : ''} />
		</div>
	</Label>
</div>
