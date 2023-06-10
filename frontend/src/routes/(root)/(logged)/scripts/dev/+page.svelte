<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import { Button, Kbd } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import LogPanel from '$lib/components/scriptEditor/LogPanel.svelte'
	import { CompletedJob, Job, JobService, Preview } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, getModifierKey, sendUserToast } from '$lib/utils'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	let testJobLoader: TestJobLoader

	// Test args input
	let args: Record<string, any> = {}
	let isValid: boolean = true

	// Test
	let testIsLoading = false
	let testJob: Job | undefined
	let pastPreviews: CompletedJob[] = []
	let validCode = true

	type LastEdit = {
		content: string
		path: string
		language: Preview.language
	}

	let currentScript: LastEdit | undefined = undefined

	let schema = emptySchema()
	const href = window.location.href
	const indexQ = href.indexOf('?')
	const searchParams = indexQ > -1 ? new URLSearchParams(href.substring(indexQ)) : undefined

	if (searchParams?.has('local')) {
		connectWs()
	}

	window.addEventListener(
		'message',
		(event) => {
			if (event.data.type == 'runTest') {
				runTest()
				return
			}
			replaceScript(event.data)
		},
		false
	)

	function connectWs() {
		const port = searchParams?.get('port') || '3001'
		try {
			const socket = new WebSocket(`ws://localhost:${port}/ws`)

			// Listen for messages
			socket.addEventListener('message', (event) => {
				replaceData(event.data)
			})

			function replaceData(msg: string) {
				let data: any | undefined = undefined
				try {
					data = JSON.parse(msg)
				} catch {
					console.log('Received invalid JSON: ' + msg)
					return
				}
				replaceScript(data)
			}
		} catch (e) {
			sendUserToast('Failed to connect to local server', true)
			console.error(e)
		}
	}

	function runTest() {
		if (!currentScript) {
			return
		}
		//@ts-ignore
		testJobLoader.runPreview(
			currentScript.path,
			currentScript.content,
			currentScript.language,
			args,
			undefined
		)
	}

	async function loadPastTests(): Promise<void> {
		if (!currentScript) {
			return
		}
		console.log('Loading past tests')
		pastPreviews = await JobService.listCompletedJobs({
			workspace: $workspaceStore!,
			jobKinds: 'preview',
			createdBy: $userStore?.username,
			scriptPathExact: currentScript.path
		})
	}

	function onKeyDown(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key == 'Enter') {
			event.preventDefault()
			runTest()
		}
	}

	let lastPath: string | undefined = undefined
	async function replaceScript(lastEdit: LastEdit) {
		currentScript = lastEdit
		if (lastPath !== lastEdit.path) {
			schema = emptySchema()
		}
		try {
			await inferArgs(lastEdit.language, lastEdit.content, schema)
			schema = schema
			lastPath = lastEdit.path
			validCode = true
		} catch (e) {
			console.error(e)
			validCode = false
		}
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<TestJobLoader
	on:done={loadPastTests}
	bind:this={testJobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

<main class="h-screen w-full">
	<div class="flex flex-col h-full">
		<div class="text-center w-full text-lg truncate py-1"
			>{currentScript?.path ?? 'Not editing a script'} {currentScript?.language ?? ''}</div
		>
		{#if !validCode}
			<div class="text-center w-full text-lg truncate py-1 text-red-500">Invalid code</div>
		{/if}
		<div class="flex justify-center pt-1">
			{#if testIsLoading}
				<Button on:click={testJobLoader?.cancelJob} btnClasses="w-full" color="red" size="xs">
					<WindmillIcon
						white={true}
						class="mr-2 text-white"
						height="20px"
						width="20px"
						spin="fast"
					/>
					Cancel
				</Button>
			{:else}
				<Button
					disabled={currentScript === undefined}
					color="dark"
					on:click={() => {
						runTest()
					}}
					btnClasses="w-full"
					size="xs"
					startIcon={{
						icon: faPlay,
						classes: 'animate-none'
					}}
				>
					{#if testIsLoading}
						Running
					{:else}
						Test&nbsp;<Kbd small>{getModifierKey()}</Kbd>
						<Kbd small><span class="text-lg font-bold">⏎</span></Kbd>
					{/if}
				</Button>
			{/if}
		</div>
		<Splitpanes horizontal class="h-full">
			<Pane size={33}>
				<div class="px-2">
					<div class="break-words relative font-sans">
						<SchemaForm compact {schema} bind:args bind:isValid />
					</div>
				</div>
			</Pane>
			<Pane size={67}>
				<LogPanel
					lang={currentScript?.language}
					previewJob={testJob}
					{pastPreviews}
					previewIsLoading={testIsLoading}
				/>
			</Pane>
		</Splitpanes>
	</div>
</main>
