<script lang="ts">
	import { onMount } from 'svelte'
	import { Terminal } from 'xterm'
	import 'xterm/css/xterm.css'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import type { ScriptLang } from '$lib/gen'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import { twMerge } from 'tailwind-merge'
	import PanelSection from './apps/editor/settingsPanel/common/PanelSection.svelte'
	import { Badge } from './common'
	import StepHistory, { type StepHistoryData } from './flows/propPicker/StepHistory.svelte'
	import Select from './apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from './DarkModeObserver.svelte'

	let container: HTMLDivElement
	let term: Terminal
	let input = ''
	type Props = {
		language?: ScriptLang
		tag: string
		width?: number
		activeWorkers: string[] | undefined
	}
	let darkMode = $state(false)

	let { language = 'bash', tag, width, activeWorkers = [tag] }: Props = $props()
	let runHistory: (StepHistoryData & { command: string; result: Record<string, any>[] })[] = $state(
		[]
	)

	let working_directory = $state('')
	let prompt = $derived(`$-${working_directory.split('/').at(-1)} `)

	async function handleCommand(command: string) {
		term.writeln('')
		try {
			let { job, result } = (await runPreviewJobAndPollResult(
				{
					workspace: $workspaceStore!,
					requestBody: {
						language,
						content: `cd ${working_directory} && ${command} > result.out`,
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
		term.write(`${prompt}`)
		input = ''
	}

	onMount(async () => {
		working_directory = `/tmp/windmill/${tag}`
		term = new Terminal({
			cursorBlink: true,
			fontSize: 14,
			theme: {
				background: '#1e1e1e',
				foreground: '#ffffff'
			},
			fontFamily: 'monospace',
			convertEol: true,
			rows: 200
		})
		term.open(container)
		printPrompt()
		term.onData((char) => {
			switch (char) {
				case '\r':
					input = input.trim()
					if (input.length === 0) {
						term.write(`\r\n${prompt}`)
						break
					}
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

<DarkModeObserver bind:darkMode />

<div class="h-full">
	<Splitpanes
		class={twMerge('!overflow-visible')}
		style={width !== undefined ? `width:${width}px; height: 100%;` : 'width: 100%; height: 100%;'}
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
						</div>
						<div class="flex flex-col gap-1 w-full">
						</div>
					</div>
				</div>
			</PanelSection>
		</Pane>
		<Pane size={75}>
			<div class="m-1">
				<div class="flex justify-start w-full mb-2">
					<Badge
						color="gray"
						class="center-center !bg-gray-300 !text-tertiary dark:!bg-gray-700 dark:!text-gray-300 !h-[32px]  rounded-r-none rounded-l-none"
						>Current worker</Badge
					>
					<Select
						class="grow shrink max-w-full"
						on:change={(e) => {
							tag = e.detail.value
							working_directory = `/tmp/windmill/${tag}`
							term.reset()
							printPrompt()
						}}
						on:clear={() => {}}
						clearable={false}
						value={tag}
						items={activeWorkers}
						inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
						containerStyles={darkMode
							? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
							: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
						portal={false}
					/>
				</div>
				<div bind:this={container}></div>
			</div>
		</Pane>
	</Splitpanes>
</div>

<style>
	:global(.xterm-screen) {
		padding: 10px;
	}
</style>
