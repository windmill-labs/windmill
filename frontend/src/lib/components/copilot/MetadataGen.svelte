<script lang="ts">
	import { getCompletion } from './lib'
	import type { NewScript } from '$lib/gen'
	import { isInitialCode } from '$lib/script_helpers'
	import { Loader2 } from 'lucide-svelte'
	import type { ChatCompletionMessageParam } from 'openai/resources'
	import { copilotInfo, metadataCompletionEnabled } from '$lib/stores'
	import Label from '../Label.svelte'
	import { createEventDispatcher } from 'svelte'

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

	let loading = false
	let abortController = new AbortController()

	let focused = false
	const updateFocus = (val) => {
		focused = val
	}

	let config: Config = configs[configName]

	async function generateContent() {
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
			content = ''
			for await (const chunk of response) {
				const toks = chunk.choices[0]?.delta?.content || ''
				content += toks
				if (el !== undefined) {
					el.style.height = 'auto'
					el.style.height = el.scrollHeight + 'px'
				}
			}
		} catch (err) {
			console.error('Could not generate summary', err)
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
		generateContent()
	}

	const dispatch = createEventDispatcher()

	$: if (content) {
		dispatch('change', { content })
	}
</script>

<div class="relative">
	<div class="flex flex-row" />
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<Label {label}>
		<div slot="header" class="flex flex-row pl-1 gap-2 items-center">
			{#if $copilotInfo.exists_openai_resource_path && $metadataCompletionEnabled}
				{#if loading}
					<Loader2 class="animate-spin text-gray-400" size={18} />
					<span class="text-xs">
						<span class="border px-1 py-0.5 rounded-md text-2xs text-bold bg-white text-black">
							ESC
						</span> to cancel
					</span>
				{:else if !content && focused}
					<span class="text-xs"
						>0
						<span class="border px-1 py-0.5 rounded-md text-2xs text-bold bg-white text-black">
							TAB
						</span> to generate
					</span>
				{/if}
			{/if}
		</div>
		<div
			on:keydown={(event) => {
				if (!$copilotInfo.exists_openai_resource_path || !$metadataCompletionEnabled) {
					return
				}
				if (event.key === 'Tab' && !loading && !content) {
					event.preventDefault()
					generateContent()
				} else if (event.key === 'Escape' && loading) {
					event.preventDefault()
					event.stopPropagation()
					abortController.abort()
				}
			}}
		>
			<slot {updateFocus} />
		</div>
	</Label>
</div>
