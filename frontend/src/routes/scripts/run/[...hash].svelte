<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Run Script ${params.hash}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import {
		canWrite,
		defaultIfEmptyString,
		displayDaysAgo,
		emptySchema,
		emptyString,
		sendUserToast,
		truncateHash
	} from '$lib/utils'
	import { ScriptService, type Script, JobService } from '$lib/gen'
	import { goto } from '$app/navigation'
	import { userStore, workspaceStore } from '$lib/stores'
	import { inferArgs } from '$lib/infer'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import { Badge, Button } from '$lib/components/common'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { faPlay, faScroll } from '@fortawesome/free-solid-svg-icons'

	const hash = $page.params.hash
	let script: Script | undefined
	let runForm: RunForm | undefined
	let isValid = true
	let can_write = false

	async function loadScript() {
		if (hash) {
			script = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
			if (script.schema == undefined) {
				script.schema = emptySchema()
				inferArgs(script.language, script.content, script.schema)
				script = script
			}
			can_write =
				script.workspace_id == $workspaceStore &&
				canWrite(script.path, script.extra_perms!, $userStore)
		} else {
			sendUserToast(`Failed to fetch script hash from URL`, true)
		}
	}

	async function runScript(scheduledForStr: string | undefined, args: Record<string, any>) {
		try {
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
			let run = await JobService.runScriptByHash({
				workspace: $workspaceStore!,
				hash,
				requestBody: args,
				scheduledFor
			})
			sendUserToast(`Job <a href='/run/${run}?workspace=${$workspaceStore}'>${run}</a> started`)
			goto('/run/' + run + '?workspace=' + $workspaceStore)
		} catch (err) {
			sendUserToast(`Could not create job: ${err}`, true)
		}
	}

	$: {
		if ($workspaceStore) {
			loadScript()
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

<CenteredPage>
	{#if script}
		<div class="flex flex-row flex-wrap justify-between gap-4 mb-6">
			<div class="w-full">
				<div class="flex flex-col mt-6 mb-2 w-full">
					<div class="flex flex-row flex-wrap w-full justify-between "
						><div class="flex flex-col">
							<h1 class="break-words py-2 mr-2">
								{defaultIfEmptyString(script.summary, script.path)}
							</h1>
							{#if !emptyString(script.summary)}
								<h2 class="font-bold pb-4">{script.path}</h2>
							{/if}
						</div>
						<div class="flex-row hidden lg:flex">
							<div>
								<Button
									startIcon={{ icon: faScroll }}
									disabled={script == undefined}
									btnClasses="mr-4"
									variant="border"
									href="/scripts/get/{script?.hash}">View script</Button
								>
								<Button
									startIcon={{ icon: faPlay }}
									disabled={runForm == undefined || !isValid}
									on:click={() => runForm?.run()}>Run (Ctrl+Enter)</Button
								>
							</div>
						</div></div
					>
					<div class="flex items-center gap-2">
						<span class="text-sm text-gray-500">
							{#if script}
								Edited {displayDaysAgo(script.created_at || '')} by {script.created_by || 'unknown'}
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
			<div class="prose text-sm box max-w-6xl w-full mb-4 mt-8">
				<SvelteMarkdown source={defaultIfEmptyString(script.description, 'No description')} />
			</div>
		</div>
	{/if}

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
			autofocus
			detailed={false}
			bind:isValid
			bind:this={runForm}
			runnable={script}
			runAction={runScript}
		/>
	{/if}
</CenteredPage>
