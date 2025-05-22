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
	import { Readline } from 'xterm-readline'
	let bashEditorDrawer: Drawer | undefined = undefined

	const WORKER_CMD_FLAG = 'worker '
	let container: HTMLDivElement
	let term: Terminal
	let input = ''
	type Props = {
		tag: string
		width?: number
		activeWorkers: string[] | undefined
	}
	let scriptPicker: Drawer | undefined = $state()
	let editor = $state<Editor | null>(null)
	let darkMode = $state(false)
	let pick_existing: 'workspace' | 'hub' = $state('workspace')
	let codeViewer: Drawer | undefined = $state()
	let filter = $state('')
	let { tag, activeWorkers = [tag] }: Props = $props()
	let code: string = $state('')
	let working_directory = $state('')
	let prompt = $derived(`$-${working_directory.split('/').at(-1)} `)
	let codeObj: { language: SupportedLanguage; content: string } | undefined = $state(undefined)

	async function handleCommand(command: string) {
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

	const rl = new Readline()

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
			rightClickSelectsWord: true
		})

		term.loadAddon(rl)

		function readLine() {
			rl.read(prompt).then(processLine)
		}

		async function processLine(text: string) {
			await handleCommand(text)
			setTimeout(readLine)
		}

		const fitAddon = new FitAddon()
		term.loadAddon(fitAddon)
		term.open(container)
		term.focus()

		fitAddon.fit()

		const resizeObserver = new ResizeObserver(() => fitAddon.fit())
		resizeObserver.observe(container)

		readLine()
	})

	function clearPrompt() {
		const buffer = term.buffer.active
		const lastLineIndex = buffer.baseY + buffer.cursorY
		for (let i = lastLineIndex; i >= 0; i--) {
			const line = buffer.getLine(i)

			if (!line) break

			const text = line.translateToString()
			const postion = text.indexOf(prompt)
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
		rl.appendHistory(command)
		term.write(command)
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
