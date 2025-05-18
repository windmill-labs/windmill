<script lang="ts">
	import { onMount } from 'svelte'
	import { Terminal } from 'xterm'
	import 'xterm/css/xterm.css'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import { ScriptService, type ScriptLang } from '$lib/gen'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import { twMerge } from 'tailwind-merge'
	import PanelSection from './apps/editor/settingsPanel/common/PanelSection.svelte'
	import { Badge } from './common'
	import Select from './apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { truncate } from '$lib/utils'

	let container: HTMLDivElement
	let term: Terminal
	let input = ''
	let items: { value: string; label: string }[]
	let history: string[] = []
	let historyIndex = 0
	type Props = {
		language?: ScriptLang
		tag: string
		width?: number
		activeWorkers: string[] | undefined
	}
	let darkMode = $state(false)

	let { language = 'bash', tag, width, activeWorkers = [tag] }: Props = $props()

	let working_directory = $state('')
	let prompt = $derived(`$-${working_directory.split('/').at(-1)} `)

	async function handleCommand(command: string) {
		term.writeln('')
		try {
			let result = (await runPreviewJobAndPollResult({
				workspace: $workspaceStore!,
				requestBody: {
					language,
					content: `${command} > result.out`,
					tag,
					args: {}
				}
			})) as any

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

	function replaceInput(newInput: string) {
		while (input.length > 0) {
			term.write('\b \b')
			input = input.slice(0, -1)
		}
		input = newInput
		term.write(input)
	}

	onMount(async () => {
		items = (
			await ScriptService.listScripts({
				workspace: $workspaceStore!,
				kinds: 'script',
				languages: 'bash'
			})
		).map((script) => ({
			value: script.path,
			label: `${script.path}${script.summary ? ` | ${truncate(script.summary, 20)}` : ''}`
		}))
		console.log({ items })
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
					history.push(input)
					historyIndex = history.length
					if (input === 'clear') {
						term.reset()
						term.write(`\r\n${prompt}`)
						input = ''
						break
					}
					handleCommand(input)
					break

				case '\u007f': //Backspace
					if (input.length > 0) {
						input = input.slice(0, -1)
						term.write('\b \b')
					}
					break

				case '\x1b[A': // Up arrow
					if (historyIndex > 0) {
						//if (historyIndex === history.length) currentBuffer = input
						historyIndex--
						replaceInput(history[historyIndex])
					}
					break

				case '\x1b[B': // Down arrow
					if (historyIndex < history.length) {
						historyIndex++
						if (historyIndex === history.length) {
							replaceInput('')
						} else {
							replaceInput(history[historyIndex])
						}
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
			<PanelSection title="Bash scripts" id="app-editor-runnable-panel">
				<div class="flex flex-col gap-1 w-full"> </div>
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
