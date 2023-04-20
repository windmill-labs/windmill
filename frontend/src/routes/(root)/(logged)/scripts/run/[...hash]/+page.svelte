<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import RunForm from '$lib/components/RunForm.svelte'
	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { Alert, Badge, Button, Kbd, Skeleton } from '$lib/components/common'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { JobService, ScriptService, type Script } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		canWrite,
		defaultIfEmptyString,
		displayDaysAgo,
		emptySchema,
		emptyString,
		getModifierKey,
		sendUserToast
	} from '$lib/utils'
	import { faEye, faPen, faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import { ArrowLeftIcon, ArrowRightIcon } from 'lucide-svelte'

	$: hash = $page.params.hash
	let script: Script | undefined
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
				if (script_by_path?.hash != script.hash) {
					topHash = script_by_path?.hash
				}
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
		}
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey || event.metaKey) {
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

	let savedInputPaneSize = tweened(0, {
		duration: 200,
		easing: cubicOut
	})

	let reloadArgs = 0
</script>

<svelte:window on:keydown={onKeyDown} />

<SplitPanesWrapper class="h-screen">
	<Splitpanes class="overflow-hidden">
		<Pane class="px-4 flex justify-center" size={100 - $savedInputPaneSize} minSize={50}>
			<div class="w-full max-w-4xl flex flex-col">
				{#if script}
					<div class="flex flex-col justify-between gap-4 mb-6">
						{#if topHash}
							<div class="mt-2" />
							<Alert type="warning" title="Not HEAD">
								This hash is not HEAD (latest non-archived version at this path) :
								<a href="/scripts/run/{topHash}">Go to the HEAD of this path</a>
							</Alert>
						{:else if script.archived}
							<div class="mt-2" />

							<Alert type="error" title="Archived">This path was archived</Alert>
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
												href="/scripts/get/{script?.hash}?workspace={$workspaceStore}"
												>Script</Button
											>
										</div>
										<div>
											<Button
												startIcon={{ icon: faPlay }}
												disabled={runForm == undefined || !isValid}
												on:click={() => runForm?.run()}
											>
												Run <Kbd class="ml-2">{getModifierKey()}+Enter</Kbd>
											</Button>
										</div>
									</div>
									<div class="flex flex-col grow">
										<h1 class="break-words py-2 mr-2">
											{defaultIfEmptyString(script.summary, script.path)}
										</h1>
										{#if !emptyString(script.summary)}
											<h2 class="font-normal text-gray-500 pb-2">{script.path}</h2>
										{/if}
									</div>
								</div>
								<div class="flex items-center gap-2">
									<span class="text-xs text-gray-500">
										{#if script}
											Edited {displayDaysAgo(script.created_at || '')} by {script.created_by ||
												'unknown'}
										{/if}
									</span>
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
								This version of this script is unable to be run because because the deployment had
								the following errors:
							</p>
							<pre class="w-full text-xs mt-2 whitespace-pre-wrap">{script.lock_error_logs}</pre>
						</div>
					{:else if script && script?.lock == undefined}
						<div
							class="bg-orange-100 border-l-4 border-orange-500 text-orange-700 p-4"
							role="alert"
						>
							<p class="font-bold">Deployment in progress</p>
							<p>Refresh this page in a few seconds.</p>
						</div>
					{:else}
						<div class="flex justify-end">
							<Button
								variant="border"
								size="xs"
								color="dark"
								on:click={() => {
									//savedInputPaneSize = savedInputPaneSize == 0 ? 30 : 0
									savedInputPaneSize.set($savedInputPaneSize === 0 ? 30 : 0)
								}}
							>
								<div class="flex flex-row gap-2 items-center">
									{$savedInputPaneSize === 0 ? 'Open input library' : 'Close input library'}
									{#if $savedInputPaneSize === 0}
										<ArrowRightIcon class="w-4 h-4" />
									{:else}
										<ArrowLeftIcon class="w-4 h-4" />
									{/if}
								</div>
							</Button>
						</div>
						{#key reloadArgs}
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
						{/key}
					{/if}
				{:else}
					<Skeleton layout={[2, [3], 1, [2], 4, [4], 3, [8]]} />
				{/if}
			</div>
		</Pane>

		<Pane size={$savedInputPaneSize}>
			<SavedInputs
				scriptHash={hash}
				{isValid}
				{args}
				on:selected_args={(e) => {
					args = JSON.parse(JSON.stringify(e.detail))
					reloadArgs += 1
				}}
			/>
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
