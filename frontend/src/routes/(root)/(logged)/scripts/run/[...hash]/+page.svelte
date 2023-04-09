<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { Alert, Badge, Button, Kbd, Skeleton } from '$lib/components/common'
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { JobService, ScriptService, type Script, type JobInput } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		canWrite,
		defaultIfEmptyString,
		displayDate,
		displayDaysAgo,
		emptySchema,
		emptyString,
		sendUserToast,
		truncateHash
	} from '$lib/utils'
	import { faEye, faPen, faPlay } from '@fortawesome/free-solid-svg-icons'

	$: hash = $page.params.hash
	let script: Script | undefined
	let job_inputs: JobInput[] = []
	let selectedArgs = {}
	let runForm: RunForm | undefined
	let isValid = true
	let can_write = false
	let topHash: string | undefined
	let args: object = {}

	async function loadScript() {
		if (hash) {
			script = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })

			if (script.schema == undefined) {
				script.schema = emptySchema()
				await inferArgs(script.language, script.content, script.schema)
				script = script
			}

			if (script.path && script.archived) {
				const script_by_path = await ScriptService.getScriptByPath({
					workspace: $workspaceStore!,
					path: script.path
				}).catch((_) => console.error('this script has no non-archived version'))
				topHash = script_by_path?.hash
			} else {
				topHash = undefined
			}

			can_write =
				script.workspace_id == $workspaceStore &&
				canWrite(script.path, script.extra_perms!, $userStore)
		} else {
			sendUserToast(`Failed to fetch script hash from URL`, true)
		}
	}

	async function loadInputHistory() {
		job_inputs = await ScriptService.getInputHistory({
			workspace: $workspaceStore!,
			hash,
			perPage: 10
		})
	}

	let loading = false
	async function runScript(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		try {
			loading = true
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
			let run = await JobService.runScriptByHash({
				workspace: $workspaceStore!,
				hash,
				invisibleToOwner,
				requestBody: args,
				scheduledFor
			})
			await goto('/run/' + run + '?workspace=' + $workspaceStore)
		} catch (err) {
			loading = false
			sendUserToast(`Could not create job: ${err.body}`, true)
		}
	}

	$: {
		if ($workspaceStore && hash) {
			loadScript()
			loadInputHistory()
		}
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey) {
					if (isValid) {
						event.preventDefault()
						runForm?.run()
					} else {
						sendUserToast('Please fix errors before running', true)
					}
				}
				break
		}
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<div class="w-full flex justify-center pb-8 pr-80">
	<div class="w-full max-w-6xl px-4 sm:px-6 md:px-8">
		{#if script}
			<div class="flex flex-col justify-between gap-4 mb-6">
				{#if topHash}
					<Alert type="warning" title="Not HEAD">
						This hash is not HEAD (latest non-archived version at this path) :
						<a href="/scripts/run/{topHash}">Go to the HEAD of this path</a>
					</Alert>
				{/if}
				<div class="w-full">
					<div class="flex flex-col mt-6 mb-2 w-full">
						<div
							class="flex flex-row-reverse w-full flex-wrap md:flex-nowrap justify-between gap-x-1 gap-y-2"
						>
							<div class="flex flex-row gap-4">
								{#if !$userStore?.operator && can_write}
									<div>
										<Button
											size="sm"
											startIcon={{ icon: faPen }}
											disabled={script == undefined}
											variant="border"
											href="/scripts/edit/{script?.hash}">Edit</Button
										>
									</div>
								{/if}
								<div class="md:pr-4">
									<Button
										size="sm"
										startIcon={{ icon: faEye }}
										disabled={script == undefined}
										variant="border"
										href="/scripts/get/{script?.hash}?workspace_id={$workspaceStore}">View</Button
									>
								</div>
								<div>
									<Button
										startIcon={{ icon: faPlay }}
										disabled={runForm == undefined || !isValid}
										on:click={() => runForm?.run()}>Run <Kbd class="ml-2">Ctrl+Enter</Kbd></Button
									>
								</div>
							</div>
							<div class="flex flex-col grow">
								<h1 class="break-words py-2 mr-2">
									{defaultIfEmptyString(script.summary, script.path)}
								</h1>
								{#if !emptyString(script.summary)}
									<h2 class="font-bold pb-4">{script.path}</h2>
								{/if}
							</div>
						</div>
						<div class="flex items-center gap-2">
							<span class="text-sm text-gray-500">
								{#if script}
									Edited {displayDaysAgo(script.created_at || '')} by {script.created_by ||
										'unknown'}
								{/if}
							</span>
							<Badge color="dark-gray">
								{truncateHash(script?.hash ?? '')}
							</Badge>
							{#if script?.is_template}
								<Badge color="blue">Template</Badge>
							{/if}
							{#if script && script.kind !== 'script'}
								<Badge color="blue">
									{script?.kind}
								</Badge>
							{/if}

							<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
						</div>
					</div>
				</div>
				{#if !emptyString(script.description)}
					<div class="prose text-sm box max-w-6xl w-full mb-4 mt-8">
						{defaultIfEmptyString(script.description, 'No description')}
					</div>
				{/if}
			</div>

			{#if script?.lock_error_logs}
				<div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4" role="alert">
					<p class="font-bold">Not deployed properly</p>
					<p>
						This version of this script is unable to be run because because the deployment had the
						following errors:
					</p>
					<pre class="w-full text-xs mt-2 whitespace-pre-wrap">{script.lock_error_logs}</pre>
				</div>
			{:else if script && script?.lock == undefined}
				<div class="bg-orange-100 border-l-4 border-orange-500 text-orange-700 p-4" role="alert">
					<p class="font-bold">Deployment in progress</p>
					<p>Refresh this page in a few seconds.</p>
				</div>
			{:else}
				<RunForm
					{loading}
					autofocus
					detailed={false}
					bind:isValid
					bind:this={runForm}
					runnable={script}
					runAction={runScript}
					viewCliRun
					isFlow={false}
					bind:args
				/>
			{/if}
		{:else}
			<Skeleton layout={[2, [3], 1, [2], 4, [4], 3, [8]]} />
		{/if}
	</div>
</div>

<div class="fixed right-0 top-0 bottom-0 w-80 h-full grid grid-rows-2 bg-gray-50 border-l">
	<div class="w-full flex flex-col gap-4 p-4">
		<h2>Previous Inputs</h2>

		<div class="w-full flex flex-col gap-2 p-2 h-full overflow-y-auto overflow-x-hidden">
			{#if job_inputs.length > 0}
				{#each job_inputs as { created_by, started_at, args }}
					<Button color="blue" btnClasses="w-full" on:click={() => (selectedArgs = args)}>
						<div class="w-full h-full items-center flex gap-4">
							<small>{displayDate(started_at)}</small>
							<small class="w-[160px] overflow-x-hidden text-ellipsis text-left">
								{created_by}
							</small>
						</div>
					</Button>
				{/each}
			{:else}
				<div class="text-center text-gray-500">No previous inputs</div>
			{/if}
		</div>
	</div>

	<div class="w-full flex flex-col gap-4 p-4 border-t">
		<h2>Preview</h2>

		<div class="w-full h-full overflow-auto">
			{#if Object.keys(selectedArgs).length > 0}
				<ObjectViewer json={selectedArgs} />
			{:else}
				<div class="text-center text-gray-500">Select an Input to preview scripts arguments</div>
			{/if}
		</div>
	</div>

	<div class="w-full flex flex-col p-4 border-t">
		<Button
			color="blue"
			btnClasses="w-full"
			on:click={() => (args = selectedArgs)}
			disabled={Object.keys(selectedArgs).length === 0}
		>
			Use Input
		</Button>
	</div>
</div>
