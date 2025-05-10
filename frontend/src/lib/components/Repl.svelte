<script lang="ts">
	import { onMount } from 'svelte'
	import { Terminal } from 'xterm'
	import 'xterm/css/xterm.css'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import type { ScriptLang } from '$lib/gen'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import { twMerge } from 'tailwind-merge'
	import { Plus } from 'lucide-svelte'
	import PanelSection from './apps/editor/settingsPanel/common/PanelSection.svelte'
	import { Button } from './common'
	import StepHistory, { type StepHistoryData } from './flows/propPicker/StepHistory.svelte'

	let container: HTMLDivElement
	let term: Terminal
	let input = ''
	type Props = {
		language?: ScriptLang
		tag?: string
		width?: number
	}
	const prompt = '$ '
	let { language = 'bash', tag, width }: Props = $props()
	let runHistory: (StepHistoryData & { command: string; result: Record<string, any>[] })[] = $state(
		[]
	)

	async function handleCommand(command: string) {
		term.writeln('')
		try {
			let { job, result } = (await runPreviewJobAndPollResult(
				{
					workspace: $workspaceStore!,
					requestBody: {
						language,
						content: `${command} > result.out`,
						tag,
						args: {}
					}
				},
				{ withJobData: true }
			)) as any
			runHistory.push({
				created_at: new Date().toISOString(),
				created_by: '',
				id: job.id,
				success: true,
				command,
				result
			})

			term.write(result)
		} catch (e) {
			term.writeln(`Error: ${e}`)
		} finally {
			printPrompt()
		}
	}

	function printPrompt() {
		term.write(`\r\n${prompt}`)
		input = ''
	}

	onMount(() => {
		term = new Terminal({
			cursorBlink: true,
			fontSize: 14,
			theme: {
				background: '#1e1e1e',
				foreground: '#ffffff'
			},
			convertEol: true
		})
		term.open(container)
		printPrompt()

		term.onData((char) => {
			switch (char) {
				case '\r':
					if (input === 'clear') {
						term.reset()
						term.write(`\r\n${prompt}`)
						input = ''
						break
					}
					handleCommand(input)
					break
				case '\u007f':
					if (input.length > 0) {
						input = input.slice(0, -1)
						term.write('\b \b')
					}
					break

				default:
					if (char >= String.fromCharCode(0x20)) {
						input += char
						term.write(char)
					}
			}
		})
	})
</script>

<Splitpanes
	class={twMerge('!overflow-visible')}
	style={width !== undefined ? `width:${width}px;` : 'width: 100%;'}
>
	<Pane size={25}>
		<PanelSection title="History" id="app-editor-runnable-panel">
			<div class="w-full flex flex-col gap-6 py-1">
				<div>
					<StepHistory
						staticInputs={runHistory}
						on:select={(e) => {
							const data = e.detail as (typeof runHistory)[number]
							if (data) {
								term.reset()
								term.write(`\r\n${prompt}${data.command}`)
								input = data.command
							}
						}}
					/>
				</div>
				<div>
					<div class="w-full flex justify-between items-center mb-1">
						<div class="text-xs text-secondary font-semibold truncate"> Bash scripts </div>
						<Button
							size="xs"
							color="light"
							variant="border"
							btnClasses="!rounded-full !p-1"
							title="Create a new background runnable"
							aria-label="Create a new background runnable"
							on:click={() => {}}
							id="create-bash-script"
						>
							<Plus size={14} class="!text-primary" />
						</Button>
					</div>
					<div class="flex flex-col gap-1 w-full">
						<div class="text-xs text-tertiary">No bash scripts </div>
					</div>
				</div>
			</div>
		</PanelSection>
	</Pane>
	<Pane size={75}>
		<!-- svelte-ignore element_invalid_self_closing_tag -->
		<div bind:this={container} class="terminal"></div>
	</Pane>
</Splitpanes>

<style>
	.terminal {
		width: 100%;
		height: 100%;
		background-color: #1e1e1e;
	}
</style>
