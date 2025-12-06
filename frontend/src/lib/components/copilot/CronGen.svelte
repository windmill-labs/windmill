<script lang="ts">
	import { ExternalLink, Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { sendUserToast } from '$lib/toast'

	import { base } from '$lib/base'
	import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
	import { copilotInfo } from '$lib/aiStore'
	import { untrack } from 'svelte'

	interface Props {
		schedule: string
		cronVersion: string
	}

	let { schedule = $bindable(), cronVersion = $bindable('v2') }: Props = $props()

	let instructions = $state('')
	let instructionsField: HTMLInputElement | undefined = $state(undefined)
	let genLoading = $state(false)
	let abortController = $state(new AbortController())

	const SYSTEM_V2 =
		"You are a helpful assistant for creating CRON schedules using both standard and extended Croner patterns. The structure is 'second minute hour dayOfMonth month dayOfWeek'. Supported modifiers: ? (wildcard), L (last day/weekday), # (nth occurrence of a weekday), and W (closest weekday). Weekdays are Sunday (0 or 7), Monday (1), Tuesday (2), Wednesday (3), Thursday (4), Friday (5), Saturday (6). Ensure syntax is valid, including optional seconds and special modifiers. You only return either the CRON string without any leading/closing quotes or an error message prefixed with 'ERROR:'."

	const SYSTEM_V1 =
		"You are a helpful assistant for creating CRON schedules. The structure is 'second minute hour dayOfMonth month dayOfWeek'. Weekdays are Sunday (1), Monday (2), Tuesday (3), Wednesday (4), Thursday (5), Friday (6), Saturday (7). You only return the CRON string without any wrapping characters. If it is invalid, you will return an error message preceded by 'ERROR:'."

	function updateSystemPrompt(version: string) {
		if (version === 'v2') {
			SYSTEM = SYSTEM_V2
		} else {
			SYSTEM = SYSTEM_V1
		}
	}

	let SYSTEM = SYSTEM_V2

	const USER = 'CRON schedule instructions: {instructions}'
	async function generateCron() {
		genLoading = true
		abortController = new AbortController()
		try {
			const messages: ChatCompletionMessageParam[] = [
				{
					role: 'system',
					content: SYSTEM
				},
				{
					role: 'user',
					content: USER.replace('{instructions}', instructions)
				}
			]

			const response = await getNonStreamingCompletion(messages, abortController)

			if (response.startsWith('ERROR:')) {
				throw response.replace('ERROR:', '').trim()
			}
			schedule = response
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Could not generate CRON schedule: ' + err, true)
			}
		} finally {
			genLoading = false
		}
	}
	$effect(() => {
		instructionsField && setTimeout(() => instructionsField?.focus(), 100)
	})
	$effect(() => {
		cronVersion
		untrack(() => {
			updateSystemPrompt(cronVersion)
		})
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
				<div class="flex w-96">
					<input
						bind:this={instructionsField}
						type="text"
						placeholder="CRON schedule description"
						bind:value={instructions}
						onkeypress={({ key }) => {
							if (key === 'Enter' && instructions.length > 0) {
								close()
								generateCron()
							}
						}}
					/>
					<Button
						size="xs"
						color="light"
						variant="contained"
						buttonType="button"
						btnClasses="!ml-2 text-ai bg-violet-100 dark:bg-gray-700"
						title="Generate CRON schedule from prompt"
						aria-label="Generate"
						iconOnly
						on:click={() => {
							close()
							generateCron()
						}}
						disabled={instructions.length == 0}
						startIcon={{ icon: Wand2 }}
					/>
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
