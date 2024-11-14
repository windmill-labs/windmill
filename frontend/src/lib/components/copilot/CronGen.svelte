<script lang="ts">
	import { ExternalLink, Wand2 } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { getNonStreamingCompletion } from './lib'
	import Popup from '../common/popup/Popup.svelte'
	import { sendUserToast } from '$lib/toast'
	import { copilotInfo } from '$lib/stores'
	import { base } from '$lib/base'

	export let schedule: string

	let instructions = ''
	let instructionsField: HTMLInputElement | undefined = undefined
	let genLoading = false
	let abortController = new AbortController()
	$: instructionsField && setTimeout(() => instructionsField?.focus(), 100)
	const SYSTEM =
		"You are a helpful assistant for creating CRON schedules using both standard and extended Croner patterns. The structure is 'second minute hour dayOfMonth month dayOfWeek'. Supported modifiers: ? (wildcard), L (last day/weekday), # (nth occurrence of a weekday), and W (closest weekday). Weekdays are Sunday (1 or 7), Sunday (1), Monday (2), Tuesday (3), Wednesday (4), Thursday (5), Friday (6), Saturday (7). Ensure syntax is valid, including optional seconds and special modifiers. You only return either the CRON string without any leading/closing quotes or an error message prefixed with 'ERROR:'."
	const USER = 'CRON schedule instructions: {instructions}'
	async function generateCron() {
		genLoading = true
		abortController = new AbortController()
		try {
			const response = await getNonStreamingCompletion(
				[
					{
						role: 'system',
						content: SYSTEM
					},
					{
						role: 'user',
						content: USER.replace('{instructions}', instructions)
					}
				],
				abortController
			)

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
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	let:close
>
	<svelte:fragment slot="button">
		<Button
			color={genLoading ? 'red' : 'light'}
			size="xs"
			nonCaptureEvent={!genLoading}
			startIcon={{ icon: Wand2 }}
			iconOnly
			title="AI Assistant"
			btnClasses="text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700"
			loading={genLoading}
			clickableWhileLoading
			on:click={genLoading ? () => abortController?.abort() : undefined}
		/>
	</svelte:fragment>
	{#if $copilotInfo.exists_openai_resource_path}
		<div class="flex w-96">
			<input
				bind:this={instructionsField}
				type="text"
				placeholder="CRON schedule description"
				bind:value={instructions}
				on:keypress={({ key }) => {
					if (key === 'Enter' && instructions.length > 0) {
						close(instructionsField || null)
						generateCron()
					}
				}}
			/>
			<Button
				size="xs"
				color="light"
				variant="contained"
				buttonType="button"
				btnClasses="!ml-2 text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700"
				title="Generate CRON schedule from prompt"
				aria-label="Generate"
				iconOnly
				on:click={() => {
					close(instructionsField || null)
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
					href="{base}/workspace_settings?tab=openai"
					target="_blank"
					class="inline-flex flex-row items-center gap-1"
					>workspace settings <ExternalLink size={16} /></a
				></p
			>
		</div>
	{/if}
</Popup>
