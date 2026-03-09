<script lang="ts">
	import { ExternalLink, Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$lib/base'
	import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
	import { copilotInfo } from '$lib/aiStore'
	import type { Schema } from '$lib/common'

	interface Props {
		args: Record<string, any>
		resourceType?: string
		resourceName?: string
		resourceDescription?: string
		resourceSchema?: Schema | undefined
		isFileset?: boolean
	}

	let {
		args = $bindable(),
		resourceType = '',
		resourceName = '',
		resourceDescription = '',
		resourceSchema = undefined,
		isFileset = false
	}: Props = $props()

	let instructions = $state('')
	let instructionsField: HTMLTextAreaElement | undefined = $state(undefined)
	let genLoading = $state(false)
	let abortController = $state(new AbortController())

	function buildSystemPrompt(): string {
		let prompt: string

		if (isFileset) {
			prompt =
				'You are a helpful assistant that generates file contents for a Windmill fileset resource. A fileset is a JSON object mapping file paths to their string contents, e.g. {"path/to/file.txt": "file content", "other/file.md": "# Title"}. You MUST return ONLY valid JSON (a flat object with string keys and string values), no markdown fences, no explanation, no extra text.'
		} else {
			prompt =
				'You are a helpful assistant that generates JSON values for Windmill resources. You MUST return ONLY valid JSON, no markdown fences, no explanation, no extra text.'
		}

		if (resourceType) {
			prompt += `\nThe resource type is "${resourceType}".`
		}

		if (!isFileset && resourceSchema?.properties) {
			const schemaDesc = Object.entries(resourceSchema.properties)
				.map(([key, prop]: [string, any]) => {
					let desc = `- "${key}": type=${prop.type || 'string'}`
					if (prop.description) desc += `, description="${prop.description}"`
					if (prop.default !== undefined) desc += `, default=${JSON.stringify(prop.default)}`
					if (prop.enum) desc += `, enum=${JSON.stringify(prop.enum)}`
					return desc
				})
				.join('\n')
			prompt += `\n\nThe resource schema has these properties:\n${schemaDesc}`

			if (resourceSchema.required?.length) {
				prompt += `\n\nRequired fields: ${resourceSchema.required.join(', ')}`
			}
		}

		return prompt
	}

	function buildUserPrompt(): string {
		let prompt = instructions

		if (resourceName) {
			prompt += `\nResource name: ${resourceName}`
		}
		if (resourceDescription) {
			prompt += `\nResource description: ${resourceDescription}`
		}

		return prompt
	}

	async function generateResource() {
		genLoading = true
		abortController = new AbortController()
		try {
			const messages: ChatCompletionMessageParam[] = [
				{
					role: 'system',
					content: buildSystemPrompt()
				},
				{
					role: 'user',
					content: buildUserPrompt()
				}
			]

			let response = await getNonStreamingCompletion(messages, abortController)

			// Strip markdown fences if present
			response = response.trim()
			if (response.startsWith('```')) {
				response = response.replace(/^```(?:json)?\n?/, '').replace(/\n?```$/, '')
			}

			const parsed = JSON.parse(response)
			args = parsed
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate resource: ' + err, true)
			}
		} finally {
			genLoading = false
		}
	}

	$effect(() => {
		instructionsField && setTimeout(() => instructionsField?.focus(), 100)
	})
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	{#snippet trigger()}
		<Button
			color={genLoading ? 'red' : 'light'}
			size="xs"
			nonCaptureEvent={!genLoading}
			startIcon={{ icon: Wand2 }}
			iconOnly
			title="AI Assistant"
			btnClasses="text-ai bg-violet-100 dark:bg-gray-700"
			loading={genLoading}
			clickableWhileLoading
			on:click={genLoading ? () => abortController?.abort() : () => {}}
		/>
	{/snippet}
	{#snippet content({ close })}
		<div class="border rounded-lg shadow-lg p-4 bg-surface">
			{#if $copilotInfo.enabled}
				<div class="flex flex-col w-80 gap-2">
					<textarea
						bind:this={instructionsField}
						placeholder="Describe the resource values to generate..."
						bind:value={instructions}
						rows={3}
						class="text-xs"
						onkeydown={(e) => {
							if (e.key === 'Enter' && !e.shiftKey && instructions.length > 0) {
								e.preventDefault()
								close()
								generateResource()
							}
						}}
					/>
					<Button
						size="xs"
						color="light"
						variant="contained"
						buttonType="button"
						btnClasses="text-ai bg-violet-100 dark:bg-gray-700"
						title="Generate resource from prompt"
						on:click={() => {
							close()
							generateResource()
						}}
						disabled={instructions.length == 0}
						startIcon={{ icon: Wand2 }}
					>
						Generate
					</Button>
				</div>
			{:else}
				<div class="block text-primary">
					<p class="text-sm"
						>Enable Windmill AI in the <a
							href="{base}/workspace_settings?tab=ai"
							target="_blank"
							class="inline-flex flex-row items-center gap-1"
							>workspace settings <ExternalLink size={16} /></a
						></p
					>
				</div>
			{/if}
		</div>
	{/snippet}
</Popover>
