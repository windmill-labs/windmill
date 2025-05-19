<script lang="ts">
	import { onDestroy, onMount } from 'svelte'
	import { Terminal } from 'xterm'
	import 'xterm/css/xterm.css'
	import { runScriptAndPollResult } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import { ScriptService, type RunScriptByPathData, type Script } from '$lib/gen'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import { twMerge } from 'tailwind-merge'
	import PanelSection from './apps/editor/settingsPanel/common/PanelSection.svelte'
	import { Badge, Button, Drawer, DrawerContent } from './common'
	import Select from './apps/svelte-select/lib/Select.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import InfiniteList from './InfiniteList.svelte'
	import { sendUserToast } from '$lib/utils'
	import { Eye, Play } from 'lucide-svelte'
	import Cell from './table/Cell.svelte'
	import Editor from './Editor.svelte'
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
	let editor = $state<Editor | null>(null)
	let darkMode = $state(false)
	let infiniteList: InfiniteList | undefined = undefined
	let selected: string = ''
	let { tag, width, activeWorkers = [tag] }: Props = $props()

	let working_directory = $state('')
	let prompt = $derived(`$-${working_directory.split('/').at(-1)} `)

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
				console.log({ command })
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
			convertEol: true
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

	onDestroy(() => {
		//resetSelected()
	})

	/*function resetSelected(dispatchEvent: boolean = true) {
		selected = undefined
		if (dispatchEvent) {
			dispatch('select', undefined)
		}
	}*/

	function handleError(error: { type: string; error: any }) {
		if (error.type === 'load') {
			sendUserToast(`Failed to load bash scripts: ${error.error}`, true)
		}
	}

	function handleSelect(bashScript: Script) {
		console.log({ bashScript })
	}

	function initLoadBashScripts() {
		const loadInputsPageFn = async (page: number, perPage: number) => {
			const bashScripts = await ScriptService.listScripts({
				workspace: $workspaceStore!,
				kinds: 'script',
				languages: 'bash',
				page,
				perPage
			})
			console.log({ bashScripts })
			return bashScripts
		}
		infiniteList?.setLoader(loadInputsPageFn)
	}
</script>

<DarkModeObserver bind:darkMode />

<Drawer bind:this={bashEditorDrawer} size="800px">
	<DrawerContent title="Bash Editor" on:close={() => bashEditorDrawer?.closeDrawer?.()}>
		<Editor
			bind:this={editor}
			code={'dad'}
			lang="bash"
			scriptLang="mysql"
			class="w-full h-full"
		/></DrawerContent
	>
</Drawer>

<div class="h-full">
	<Splitpanes
		class={twMerge('!overflow-visible')}
		style={width !== undefined ? `width:${width}px; height: 100%;` : 'width: 100%; height: 100%;'}
	>
		<Pane size={25}>
			<PanelSection title="Bash scripts" id="app-editor-runnable-panel">
				<div class="flex flex-col gap-1 w-full">
					<InfiniteList
						selectedItemId={selected}
						on:error={(e) => handleError(e.detail)}
						on:select={(e) => handleSelect(e.detail)}
						bind:this={infiniteList}
					>
						<svelte:fragment let:item>
							<Cell>
								<div class="flex flex-row">
									<input class="truncate" readonly disabled value={item.path} />
									<Button
										variant="contained"
										size="xs2"
										color="light"
										btnClasses="bg-transparent hover:bg-surface"
										on:click={() => {
											bashEditorDrawer?.openDrawer()
										}}
									>
										<Eye size={16} />
									</Button>
									<Button
										variant="contained"
										size="xs2"
										color="light"
										btnClasses="bg-transparent hover:bg-surface"
									>
										<Play size={16} />
									</Button>
								</div>
							</Cell>
						</svelte:fragment>
						<svelte:fragment slot="empty">
							<div class="text-center text-xs text-tertiary my-2">No bash scripts</div>
						</svelte:fragment>
					</InfiniteList>
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
		padding-left: 10px;
	}
</style>
