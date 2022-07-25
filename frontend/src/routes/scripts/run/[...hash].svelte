<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Run Script ${params.hash}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { emptySchema, sendUserToast } from '$lib/utils'
	import { ScriptService, type Script, JobService } from '$lib/gen'
	import { goto } from '$app/navigation'
	import { workspaceStore } from '$lib/stores'
	import { inferArgs } from '$lib/infer'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'

	const hash = $page.params.hash
	let script: Script | undefined

	async function loadScript() {
		if (hash) {
			script = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
			if (script.schema == undefined) {
				script.schema = emptySchema()
				inferArgs(script.language, script.content, script.schema)
				script = script
			}
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
			sendUserToast(`Job <a href='/run/${run}'>${run}</a> was created.`)
			goto('/run/' + run)
		} catch (err) {
			sendUserToast(`Could not create job: ${err}`, true)
		}
	}

	$: {
		if ($workspaceStore) {
			loadScript()
		}
	}
</script>

<CenteredPage>
	<PageHeader title="Run script {script?.path ?? '...'}">
		<a href="/scripts/get/{script?.hash}">View script {script?.path} at hash {script?.hash}</a>
	</PageHeader>
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
			<p class="font-bold">Deployement in progress</p>
			<p>Refresh this page in a few seconds.</p>
		</div>
	{:else}
		<RunForm runnable={script} runAction={runScript} />
	{/if}
</CenteredPage>
