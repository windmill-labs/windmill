<script lang="ts">
	import { onMount } from 'svelte'
	import { Terminal } from 'xterm'
	import 'xterm/css/xterm.css'
	import { pollJobResult, runScript } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import { Badge, Button, Drawer, DrawerContent, Skeleton } from './common'
	import DarkModeObserver from './DarkModeObserver.svelte'
	import { Eye, Library, Play, RefreshCw, Square } from 'lucide-svelte'
	import Editor from './Editor.svelte'
	import WorkspaceScriptPicker from './flows/pickers/WorkspaceScriptPicker.svelte'
	import ToggleHubWorkspace from './ToggleHubWorkspace.svelte'
	import type { SupportedLanguage } from '$lib/common'
	import HighlightCode from './HighlightCode.svelte'
	import PickHubScript from './flows/pickers/PickHubScript.svelte'
	import { getScriptByPath } from '$lib/scripts'
	import { FitAddon } from '@xterm/addon-fit'
	import { Readline } from 'xterm-readline'
	import Tooltip from './Tooltip.svelte'
	import { JobService, type QueuedJob } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Select from './select/Select.svelte'
	import { emptyString } from '$lib/utils'

	let container: HTMLDivElement
	let term: Terminal
	let input = ''
	type Props = {
		tag: string
		width?: number
	}
	let scriptPicker: Drawer | undefined = $state()
	let editor = $state<Editor | null>(null)
	let darkMode = $state(false)
	let pick_existing: 'workspace' | 'hub' = $state('workspace')
	let jobId = $state('')
	let selectedJobId = $state('')
	let codeViewer: Drawer | undefined = $state()
	let filter = $state('')
	let { tag }: Props = $props()
	let code: string = $state('')
	let working_directory = $state('~')
	let homeDirectory: string = '~'
	let pendingsJobs: Array<QueuedJob> = $state([])
	let loadingPendingJobs = $state(false)
	let isCancelingJob = $state(false)
	let prompt = $derived(
		`$-${working_directory === '/' ? '/' : working_directory.split('/').pop()} `
	)
	let codeObj: { language: SupportedLanguage; content: string } | undefined = $state(undefined)
	function getNewWorkingDirectoryPath(currentDir: string, newPath: string): string {
		if (newPath.startsWith('/') || newPath.startsWith('~')) {
			return newPath
		}

		return currentDir.replace(/\/+$/, '') + '/' + newPath
	}

	function ensureTrailingLineBreak(input: any): boolean {
		if (typeof input !== 'string') {
			return false
		}
		return input.endsWith('\r\n') || input.endsWith('\n')
	}

	function isSimpleCdCommand(command: string): boolean {
		const trimmed = command.trim()

		// Matches:
		// - Starts with "cd"
		// - Followed by any number of valid args (quoted or not)
		// - No use of &, |, ; outside quotes
		// - No other commands
		const cdRegex = /^cd(\s+("[^"]*"|'[^']*'|[^\s"'&|;]+))*\s*$/

		return cdRegex.test(trimmed)
	}

	async function handleCommand(command: string) {
		try {
			const trimmedCommand = command.trim()

			if (trimmedCommand.length === 0) return

			const isOnlyCdCommand = isSimpleCdCommand(trimmedCommand)
			let wDirectory = working_directory
			if (isOnlyCdCommand) {
				const parts = trimmedCommand.split(' ')
				if (parts.length > 1) {
					const path = parts.slice(1).join(' ')
					const newPath = getNewWorkingDirectoryPath(working_directory, path)
					wDirectory = newPath
				} else {
					wDirectory = homeDirectory
				}
			}

			jobId = await runScript({
				workspace: $workspaceStore!,
				requestBody: {
					language: 'bash',
					content: `(cd ${wDirectory} && ${isOnlyCdCommand ? 'pwd' : `${trimmedCommand}`}) > result.out`,
					tag,
					args: {}
				}
			})

			let result: any = await pollJobResult(jobId, $workspaceStore!)

			if (isOnlyCdCommand) {
				working_directory = (result as string).replace(/(\r\n|\n|\r)/g, '')
				result = ''
			} else if (!ensureTrailingLineBreak(result)) {
				result += '\r\n'
			}
			rl.write(result)
		} catch (e) {
			term.writeln(`Error: ${e}`)
		}
	}

	const rl = new Readline()

	async function listPendingJobs() {
		try {
			loadingPendingJobs = true
			pendingsJobs = await JobService.listQueue({
				workspace: $workspaceStore!,
				tag,
				running: true
			})
		} catch (error) {
			sendUserToast(error.body || error.message)
		} finally {
			loadingPendingJobs = false
		}
	}

	async function listPendingJobsAndUpdateSelectedJobid() {
		await listPendingJobs()
		if (!pendingsJobs.find((pendingJob) => pendingJob.id === selectedJobId)) {
			selectedJobId = ''
		}
	}

	async function cancelJob(jobId: string) {
		try {
			await JobService.cancelQueuedJob({
				workspace: $workspaceStore!,
				id: jobId,
				requestBody: {}
			})
		} catch (err) {
			sendUserToast(err.body || err.message, true)
		}
	}

	onMount(async () => {
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

		rl.setCtrlCHandler(async () => {
			await cancelJob(jobId)
			rl.read(prompt).then(processLine)
		})

		const fitAddon = new FitAddon()
		term.loadAddon(fitAddon)
		term.open(container)
		term.focus()

		fitAddon.fit()

		const resizeObserver = new ResizeObserver(() => fitAddon.fit())
		resizeObserver.observe(container)

		//only used to silence unused variable warning from svelte compiler
		editor?.show()
		readLine()
	})

	function clearPrompt() {
		const buffer = term.buffer.active
		const lastLineIndex = buffer.baseY + buffer.cursorY
		for (let i = lastLineIndex; i >= 0; i--) {
			const line = buffer.getLine(i)

			if (!line) break

			const text = line.translateToString()
			const position = text.indexOf(prompt)
			if (position !== -1) {
				const x = position + prompt.length + 1
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
		codeObj = await getScriptByPath(e.detail.path ?? '')
		codeViewer?.openDrawer?.()
	}

	async function replacePromptWithCommand(command: string) {
		clearPrompt()
		if (!ensureTrailingLineBreak(command)) {
			command += '\r\n'
		}
		input = command
		rl.appendHistory(command)
		term.write(command)
		await handleCommand(input)
		term.write(prompt)
	}

	listPendingJobs()
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

<div class="h-screen flex flex-col">
	<div class="m-1 flex flex-col gap-2">
		<div class="flex flex-col gap-1">
			{#if pendingsJobs.length === 0}
				<Button
					loading={loadingPendingJobs}
					variant="default"
					wrapperClasses="self-stretch"
					on:click={async () => {
						await listPendingJobs()
						if (pendingsJobs.length === 0) {
							sendUserToast('No pending ssh jobs found')
						}
					}}
					startIcon={{ icon: RefreshCw }}
				>
					Load pending ssh jobs</Button
				>
			{:else}
				<div class="flex gap-1">
					<Select
						loading={loadingPendingJobs}
						class="grow shrink"
						bind:value={selectedJobId}
						items={pendingsJobs.map((pendingJob) => ({ value: pendingJob.id }))}
						placeholder="Choose a pending job id"
						clearable
						disablePortal
					/>
					<Button
						variant="default"
						disabled={emptyString(selectedJobId)}
						wrapperClasses="self-stretch"
						on:click={listPendingJobsAndUpdateSelectedJobid}
						startIcon={{ icon: RefreshCw }}
						iconOnly
					/>
					<Button
						size="xs"
						variant="default"
						disabled={emptyString(selectedJobId)}
						startIcon={{ icon: Eye }}
						on:click={async () => {
							const jobId = pendingsJobs.find((pendingsJob) => pendingsJob.id === selectedJobId)?.id
							if (jobId) {
								const job = await JobService.getJob({ workspace: $workspaceStore!, id: jobId })
								codeObj = {
									content: job.raw_code ?? '',
									language: job.language ?? 'bash'
								}
								codeViewer?.openDrawer()
							} else {
								pendingsJobs = pendingsJobs.filter((pendingJob) => pendingJob.id !== selectedJobId)
								selectedJobId = ''
							}
						}}
						iconOnly
					/>
					<Button
						loading={isCancelingJob}
						disabled={emptyString(selectedJobId)}
						size="xs"
						variant="default"
						startIcon={{ icon: Square }}
						on:click={async () => {
							try {
								isCancelingJob = true
								await cancelJob(selectedJobId)
								sendUserToast('Job cancelled successfully')
								pendingsJobs = pendingsJobs.filter((pendingJob) => pendingJob.id !== selectedJobId)
								selectedJobId = ''
							} catch (error) {
								sendUserToast(error.body || error.message, true)
								await listPendingJobsAndUpdateSelectedJobid()
							} finally {
								isCancelingJob = false
							}
						}}
						iconOnly
					/>
				</div>
			{/if}
		</div>

		<div>
			<div class="flex justify-start w-full mb-2">
				<div class="flex flex-row">
					<Badge
						color="gray"
						class="relative center-center !bg-gray-300 !text-primary dark:!bg-gray-700 dark:!text-gray-300 !h-[40px] rounded-r-none rounded-l-none"
					>
						Full path

						<Tooltip
							class="absolute top-0.5"
						>Commands run in the default directory. Run a standalone ‘cd’ to change it. Chained or invalid ‘cd’ commands won’t apply.</Tooltip>
					</Badge>
				</div>
				<input type="text" disabled bind:value={working_directory} />
			</div>

			<div bind:this={container}></div>
		</div>
	</div>
	<div class="flex flex-col h-full gap-1 mt-2">
		<div class="flex flex-row w-full justify-between">
			<div class="flex flex-row gap-2">
				<Button
					unifiedSize="md"
					variant="default"
					startIcon={{ icon: Play }}
					title="Run bash script"
					on:click={async () => {
						await replacePromptWithCommand(code)
					}}
				>
					Run
				</Button>
				<Button
					unifiedSize="md"
					variant="subtle"
					on:click={scriptPicker.openDrawer}
					startIcon={{ icon: Library }}
					title="Explore other scripts"
				>
					Library
				</Button>
			</div>
		</div>
		<Editor
			bind:this={editor}
			useWebsockets={false}
			bind:code
			scriptLang="bash"
			class="w-full h-full"
		/>
	</div>
</div>

<style>
	:global(.xterm-screen) {
		padding-left: 10px;
	}
</style>
