<script lang="ts">
	import { onMount } from 'svelte'
	import { Terminal } from 'xterm'
	import 'xterm/css/xterm.css'
	import { runScriptAndPollResult } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import { Badge, Button, Drawer, DrawerContent, Skeleton } from './common'
	import Select from './apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import InfiniteList from './InfiniteList.svelte'
	import { Library, Play } from 'lucide-svelte'
	import Editor from './Editor.svelte'
	import { ScriptService, type RunScriptByPathData } from '$lib/gen'
	import WorkspaceScriptPicker from './flows/pickers/WorkspaceScriptPicker.svelte'
	import ToggleHubWorkspace from './ToggleHubWorkspace.svelte'
	import type { SupportedLanguage } from '$lib/common'
	import HighlightCode from './HighlightCode.svelte'
	import PickHubScript from './flows/pickers/PickHubScript.svelte'
	import { getScriptByPath } from '$lib/scripts'
	import { FitAddon } from '@xterm/addon-fit'
	let bashEditorDrawer: Drawer | undefined = undefined

	const WORKER_CMD_FLAG = 'worker '
	let container: HTMLDivElement
	let term: Terminal
	let input = ''
	let history: string[] = []
	let historyIndex = 0
	type Props = {
		tag: string
		width?: number
		activeWorkers: string[] | undefined
	}
	let scriptPicker: Drawer | undefined = $state()
	let editor = $state<Editor | null>(null)
	let darkMode = $state(false)
	let infiniteList: InfiniteList | undefined = undefined
	let pick_existing: 'workspace' | 'hub' = $state('workspace')
	let codeViewer: Drawer | undefined = $state()
	let filter = $state('')
	let { tag, activeWorkers = [tag] }: Props = $props()
	let code: string = $state('')
	let working_directory = $state('')
	let prompt = $derived(`$-${working_directory.split('/').at(-1)} `)
	let codeObj: { language: SupportedLanguage; content: string } | undefined = $state(undefined)

	async function handleCommand(command: string) {
		term.writeln('')
		try {
			let result: any

			if (command.startsWith(WORKER_CMD_FLAG)) {
				const cmd = command.substring(WORKER_CMD_FLAG.length).trim()
				if (cmd.startsWith('run')) {
					const args = cmd.substring(3).trim().split(' ')

					if (args.length === 0) {
						throw Error('Missing path of script to run by the worker')
					}
					const scriptPath = args[0]
					const script = await ScriptService.getScriptByPath({
						workspace: $workspaceStore!,
						path: scriptPath
					})

					if (script.language !== 'bash') {
						throw new Error('Worker are only allowed to run bash script')
					}

					const data: RunScriptByPathData = {
						workspace: $workspaceStore!,
						path: scriptPath,
						tag,
						requestBody: {}
					}
					result = await runScriptAndPollResult(data)
				} else {
					throw new Error(
						`Unknown worker command: "${cmd}". Use "worker --help" to see available commands.`
					)
				}
			} else {
				result = await runScriptAndPollResult({
					workspace: $workspaceStore!,
					requestBody: {
						language: 'bash',
						content: `${command} > result.out`,
						tag,
						args: {}
					}
				})
			}

			term.write(result)
		} catch (e) {
			term.writeln(`Error: ${e}`)
		} finally {
			printPrompt()
		}
	}

	function printPrompt() {
		term.write(prompt)
		input = ''
	}

	function replaceInput(newInput: string) {
		clearPrompt()
		input = newInput
		term.write(input)
	}

	onMount(async () => {
		initLoadBashScripts()
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
			rightClickSelectsWord: true
		})

		const fitAddon = new FitAddon()
		term.loadAddon(fitAddon)

		term.open(container)

		fitAddon.fit()
		printPrompt()

		const resizeObserver = new ResizeObserver(() => fitAddon.fit())
		resizeObserver.observe(container)
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
					if ((input.length === 1 && input.at(0) !== ' ') || input.length > 1) {
						const buffer = term.buffer.active
						const cursorX = buffer.cursorX
						const cursorY = buffer.cursorY

						input = input.slice(0, -1)

						if (cursorX === 0 && cursorY > 0) {
							const prevLine = buffer.getLine(cursorY - 1)
							const prevLineLength = prevLine ? prevLine.length : term.cols
							term.write(`\x1b[1A\x1b[${prevLineLength}C`)
						}

						term.write('\x1b[D\x1b[P')
					}
					console.log({ input })

					break

				case '\x1b[A': // Up arrow
					if (historyIndex > 0) {
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

	function initLoadBashScripts() {
		const loadInputsPageFn = async (page: number, perPage: number) => {
			const bashScripts = await ScriptService.listScripts({
				workspace: $workspaceStore!,
				kinds: 'script',
				languages: 'bash',
				page,
				perPage
			})
			return bashScripts
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}

	function clearPrompt() {
		const buffer = term.buffer.active
		const lastLineIndex = buffer.baseY + buffer.cursorY
		for (let i = lastLineIndex; i >= 0; i--) {
			const line = buffer.getLine(i)

			if (!line) break

			const text = line.translateToString()
			const postion = text.indexOf(prompt)
			console.log(text, i)
			if (postion !== -1) {
				const x = postion + prompt.length + 1
				term.write(`\x1b[${x}G`)

				const numSpaces = text.length - x
				term.write(' '.repeat(numSpaces))

				term.write(`\x1b[${x}G`)
				break
			} else {
				term.write('\x1b[2K\r')
			}

			term.write(`\x1b[1A`)
		}
	}

	async function onScriptPick(e: { detail: { path: string } }) {
		codeObj = undefined
		codeViewer?.openDrawer?.()
		codeObj = await getScriptByPath(e.detail.path ?? '')
	}

	function replacePromptWithCommand(command: string) {
		clearPrompt()
		input = command
		term.write(command)
		history.push(command)
		historyIndex = history.length
		handleCommand(input)
	}
</script>

<DarkModeObserver bind:darkMode />

<Drawer bind:this={codeViewer} size="600px">
	<DrawerContent title="Code" on:close={codeViewer.closeDrawer}>
		{#if codeObj}
			<HighlightCode language={codeObj?.language} code={codeObj?.content} />
		{:else}
			<Skeleton layout={[[40]]} />
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={scriptPicker} size="900px">
	<DrawerContent title="Code" on:close={scriptPicker.closeDrawer}>
		{#if pick_existing == 'hub'}
			<PickHubScript bind:filter kind={'script'} on:pick={onScriptPick}>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</PickHubScript>
		{:else}
			<WorkspaceScriptPicker bind:filter kind={'script'} on:pick={onScriptPick}>
				<ToggleHubWorkspace bind:selected={pick_existing} />
			</WorkspaceScriptPicker>
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:this={bashEditorDrawer} size="800px">
	<DrawerContent title="Bash Editor" on:close={() => bashEditorDrawer?.closeDrawer?.()}>
		<Editor
			bind:this={editor}
			{code}
			lang="bash"
			scriptLang="bash"
			class="w-full h-full"
		/></DrawerContent
	>
</Drawer>

<div class="h-screen flex flex-col">
	<div class="m-1">
		<div class="flex flex-col">
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
			<div class="flex justify-start w-full mb-2">
				<Badge
					color="gray"
					class="center-center !bg-gray-300 !text-tertiary dark:!bg-gray-700 dark:!text-gray-300 !h-[32px]  rounded-r-none rounded-l-none"
					>Full path</Badge
				>
				<input type="text" bind:value={working_directory} />
			</div>
		</div>

		<div bind:this={container}></div>
	</div>
	<div class="flex flex-col h-full gap-1 mt-2">
		<div class="flex flex-row w-full justify-between">
			<div class="flex flex-row">
				<Button
					btnClasses="!font-medium text-tertiary "
					size="xs"
					spacingSize="md"
					color="light"
					startIcon={{ icon: Play }}
					title="Run bash script"
					on:click={() => {
						replacePromptWithCommand(code)
					}}
				>
					Run
				</Button>
				<Button
					btnClasses="!font-medium text-tertiary "
					size="xs"
					spacingSize="md"
					color="light"
					on:click={scriptPicker.openDrawer}
					startIcon={{ icon: Library }}
					title="Explore other scripts"
				>
					Library
				</Button>
			</div>
		</div>
		<Editor bind:this={editor} bind:code lang="bash" scriptLang="bash" class="w-full h-full" />
	</div>
</div>

<style>
	:global(.xterm-screen) {
		padding-left: 10px;
	}
</style>
