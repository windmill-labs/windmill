<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '../common'

	import { getNonStreamingCompletion } from './lib'
	import { sendUserToast } from '$lib/toast'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { copilotInfo } from '$lib/stores'

	import { autoPlacement } from '@floating-ui/core'
	import { ExternalLink, HistoryIcon, Wand2 } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	// state
	let funcDesc: string = ''
	let genLoading: boolean = false
	let input: HTMLInputElement | undefined
	let abortController: AbortController | undefined = undefined

	const dispatch = createEventDispatcher()
	async function onGenerate() {
		if (funcDesc?.length <= 0) {
			return
		}
		savePrompt()
		genLoading = true
		abortController = new AbortController()
		try {
			const res = await getNonStreamingCompletion(
				[
					{
						role: 'system',
						content:
							'Generate a regex pattern that one can use in a javascript Regex object. Output only the regex itself without any wrapping characters including the / characters. The regex should match the following:'
					},
					{
						role: 'user',
						content: funcDesc
					}
				],
				abortController
			)
			dispatch('gen', { res: res, prompt: funcDesc })
			funcDesc = ''
		} catch (err) {
			if (!abortController.signal.aborted) {
				sendUserToast('Failed to generate regex: ' + err, true)
			}
		} finally {
			genLoading = false
		}
	}

	$: input && setTimeout(() => input?.focus(), 100)

	let promptHistory: string[] = JSON.parse(getPromptsRegex() || '[]')

	function getPromptsRegex(): string | undefined {
		try {
			return localStorage.getItem('prompts-regex') ?? undefined
		} catch (e) {
			console.error('error interacting with local storage', e)
		}
		return undefined
	}

	function savePrompt() {
		if (promptHistory.includes(funcDesc)) {
			return
		}
		promptHistory.unshift(funcDesc)
		while (promptHistory.length > 5) {
			promptHistory.pop()
		}
		try {
			localStorage.setItem('prompts-regex', JSON.stringify(promptHistory))
		} catch (e) {
			console.error('error interacting with local storage', e)
		}
	}

	function clearPromptHistory() {
		promptHistory = []
		try {
			localStorage.setItem('prompts-regex', JSON.stringify(promptHistory))
		} catch (e) {
			console.error('error interacting with local storage', e)
		}
	}
</script>

<Popover
	floatingConfig={{
		middleware: [
			autoPlacement({
				allowedPlacements: ['bottom-start', 'bottom-end', 'top-start', 'top-end', 'top', 'bottom']
			})
		]
	}}
>
	<svelte:fragment slot="trigger">
		<Button
			title="Generate regexes from prompt"
			btnClasses="text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700"
			size="sm"
			color={genLoading ? 'red' : 'light'}
			spacingSize="md"
			startIcon={{ icon: Wand2 }}
			loading={genLoading}
			propagateEvent
			clickableWhileLoading
			on:click={genLoading ? () => abortController?.abort() : () => {}}
		/>
	</svelte:fragment>
	<svelte:fragment slot="content" let:close>
		<div class="block text-primary p-4">
			{#if $copilotInfo.enabled}
				<div class="flex flex-col gap-4">
					<div class="flex w-96">
						<input
							type="text"
							bind:this={input}
							bind:value={funcDesc}
							on:keypress={({ key }) => {
								if (key === 'Enter' && funcDesc?.length > 0) {
									close()
									onGenerate()
								}
							}}
							placeholder={'Describe what the regex should doww'}
						/>
						<Button
							size="xs"
							color="light"
							buttonType="button"
							btnClasses="!ml-2 text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700"
							aria-label="Generate"
							on:click={() => {
								close()
								onGenerate()
							}}
							disabled={funcDesc?.length <= 0}
							iconOnly
							startIcon={{ icon: Wand2 }}
						/>
					</div>
					{#if promptHistory.length > 0}
						<div class="w-96 flex flex-col gap-1">
							{#each promptHistory as p}
								<Button
									size="xs2"
									color="light"
									btnClasses="justify-start overflow-x-scroll no-scrollbar"
									startIcon={{ icon: HistoryIcon, classes: 'shrink-0' }}
									on:click={() => {
										funcDesc = p
									}}>{p}</Button
								>
							{/each}
							<button
								class="underline text-xs text-start px-2 text-secondary font-normal"
								on:click={clearPromptHistory}>clear history</button
							>
						</div>
					{/if}
				</div>
			{:else}
				<p class="text-sm">
					Enable Windmill AI in the <a
						href="{base}/workspace_settings?tab=ai"
						target="_blank"
						class="inline-flex flex-row items-center gap-1"
					>
						workspace settings <ExternalLink size={16} />
					</a>
				</p>
			{/if}
		</div>
	</svelte:fragment>
</Popover>
