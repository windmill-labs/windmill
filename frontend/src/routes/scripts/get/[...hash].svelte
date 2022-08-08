<script context="module">
	export function load({ params }) {
		return {
			stuff: { title: `Script ${params.hash}` }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { ScriptService, type Script } from '$lib/gen'
	import {
		truncateHash,
		sendUserToast,
		displayDaysAgo,
		canWrite,
		defaultIfEmptyString,
		scriptToHubUrl
	} from '$lib/utils'
	import Icon from 'svelte-awesome'
	import {
		faPlay,
		faEdit,
		faArchive,
		faList,
		faTrash,
		faCalendar,
		faShare,
		faSpinner,
		faGlobe
	} from '@fortawesome/free-solid-svg-icons'
	import Highlight from 'svelte-highlight'
	import typescript from 'svelte-highlight/languages/typescript'
	import python from 'svelte-highlight/languages/python'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { onDestroy } from 'svelte'

	let script: Script | undefined
	let topHash: string | undefined
	let can_write = false
	let deploymentInProgress = false
	let intervalId: NodeJS.Timer

	let shareModal: ShareModal

	$: {
		if ($workspaceStore) {
			loadScript($page.params.hash)
		}
	}

	async function deleteScript(hash: string): Promise<void> {
		try {
			await ScriptService.deleteScriptByHash({ workspace: $workspaceStore!, hash })
			loadScript(hash)
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not delete this script ${err.body}`, true)
		}
	}

	async function archiveScript(hash: string): Promise<void> {
		await ScriptService.archiveScriptByHash({ workspace: $workspaceStore!, hash })
		loadScript(hash)
	}

	async function syncer(): Promise<void> {
		if (script?.hash) {
			const status = await ScriptService.getScriptDeploymentStatus({
				workspace: $workspaceStore!,
				hash: script?.hash!
			})
			if (status.lock != undefined || status.lock_error_logs != undefined) {
				deploymentInProgress = false
				script.lock = status.lock
				script.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			}
		}
	}

	async function loadScript(hash: string): Promise<void> {
		try {
			script = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
		} catch {
			script = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: hash })
			hash = script.hash
		}
		can_write =
			script.workspace_id == $workspaceStore &&
			canWrite(script.path, script.extra_perms!, $userStore)
		if (script.path && script.archived) {
			const script_by_path = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: script.path
			}).catch((_) => console.error('this script has no non-archived version'))
			topHash = script_by_path?.hash
		} else {
			topHash = undefined
		}
		intervalId && clearInterval(intervalId)
		deploymentInProgress = script.lock == undefined && script.lock_error_logs == undefined
		if (deploymentInProgress) {
			intervalId = setInterval(syncer, 500)
		}
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})
</script>

<CenteredPage>
	<div class="flex flex-row justify-between">
		<h1>
			{script?.path ?? 'Loading...'}
			<span class="whitespace-nowrap">
				<a href="/scripts/get/{script?.hash}"
					><span class="commit-hash">{truncateHash(script?.hash ?? '')}</span></a
				>
				<Tooltip>Each script version has an immutable hash.</Tooltip>
			</span>
			{#if script?.is_template}
				<span class="mx-2 bg-blue-500 rounded-md bg-opacity-25 text-sm font-normal px-1 py-px"
					>Template</span
				>
			{/if}
			{#if script?.is_trigger}
				<span class="mx-2 bg-blue-500 rounded-md bg-opacity-25 text-sm font-normal px-1 py-px"
					>Trigger</span
				>
			{/if}
			<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
			{#if deploymentInProgress}
				<span class="bg-yellow-200 text-gray-700 text-xs rounded px-1 mx-3">
					Deployment in progress <Icon class="animate-spin" data={faSpinner} scale={0.8} />
				</span>
			{/if}
		</h1>

		{#if script}
			<div class="flex flex-row-reverse px-6">
				<Dropdown
					dropdownItems={[
						{
							displayName: 'Use as template',
							icon: faEdit,
							href: `/scripts/add?template=${script.path}`
						},
						{
							displayName: 'Share',
							icon: faShare,
							action: () => {
								shareModal.openModal()
							},
							disabled: !can_write
						},
						{
							displayName: 'Schedule',
							icon: faCalendar,
							href: `/schedule/add?path=${script.path}`
						},
						{
							displayName: 'Archive',
							icon: faArchive,
							type: 'delete',
							action: () => {
								script?.hash && archiveScript(script.hash)
							},
							disabled: script.archived || !can_write
						},
						{
							displayName: 'Delete',
							icon: faTrash,
							type: 'delete',
							action: () => {
								script?.hash && deleteScript(script.hash)
							},
							disabled: script.deleted || !($userStore?.is_admin ?? false)
						}
					]}
				/>
				<div class="px-1">
					<a
						target="_blank"
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href={scriptToHubUrl(
							script.content,
							script.summary,
							script.description ?? '',
							script.is_trigger
						).toString()}
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faGlobe} scale={0.5} />
							<span class="pl-1">Publish to Hub</span>
						</div>
					</a>
				</div>
				<div class="px-1">
					<a
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href="/runs/{script.path}"
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faList} scale={0.5} />
							<span class="pl-1">View runs</span>
						</div>
					</a>
				</div>
				{#if !topHash}
					<div class="px-1">
						<a
							class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
							href="/scripts/edit/{script.hash}?step=2"
							class:disabled={!can_write}
						>
							<div class="inline-flex items-center justify-center">
								<Icon class="text-blue-500" data={faEdit} scale={0.5} />
								<span class="pl-1">Edit</span>
							</div>
						</a>
					</div>
				{/if}

				<div class="px-1">
					<a
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href="/scripts/run/{script.hash}"
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faPlay} scale={0.5} />
							<span class="pl-1">Run</span>
						</div>
					</a>
				</div>
			</div>
		{/if}
	</div>

	<ShareModal bind:this={shareModal} kind="script" path={script?.path ?? ''} />

	<div class="grid grid-cols-1 gap-6 max-w-7xl pb-6">
		{#if script === undefined}
			<p>loading</p>
		{:else}
			<p class="text-sm">Edited {displayDaysAgo(script.created_at ?? '')} by {script.created_by}</p>

			<h2>{script.summary}</h2>

			<div class="prose">
				<SvelteMarkdown source={defaultIfEmptyString(script.description, 'No description')} />
			</div>

			{#if script.lock_error_logs}
				<div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4" role="alert">
					<p class="font-bold">Error deploying this script</p>
					<p>This script has not been deployed successfully because of the following errors:</p>
					<pre class="w-full text-xs mt-2 whitespace-pre-wrap">{script.lock_error_logs}</pre>
				</div>
			{/if}
			{#if topHash}
				<div class="bg-orange-100 border-l-4 border-orange-500 text-orange-700 p-4" role="alert">
					<p class="font-bold">Not HEAD</p>
					<p>
						This hash is not HEAD (latest non archived verson at this path) :
						<a href="/scripts/get/{topHash}">Go to the HEAD of this path</a>
					</p>
				</div>
			{/if}
			{#if script.archived}
				<div class="bg-red-100 border-l-4 border-red-500 text-orange-700 p-4" role="alert">
					<p class="font-bold">Archived</p>
					<p>This version was archived</p>
				</div>
			{/if}
			{#if script.deleted}
				<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4" role="alert">
					<p class="font-bold">Deleted</p>
					<p>The content of this script was deleted (by an admin, no less)</p>
				</div>
			{/if}

			<div>
				<h3>
					Current hash <Tooltip
						>The hash is an immutable and perpetual unique identifier for this version of this
						script. The history of all hashes of a script constitute its lineage. This mechanism
						shares some of the principles of git which identify each commit with an equivalent hash</Tooltip
					>
				</h3>
				<p class="text-gray-700">
					<a href="/scripts/get/{script?.hash}">{script?.hash}</a>
				</p>
				<h3 class="whitespace-nowrap mt-2">
					Webhook to run this script and get job's uuid as response
					<Tooltip
						>Send a POST http request with a token as bearer token and the args respecting the
						corresponding jsonschema as payload. To create a permanent token, go to your user
						setting by clicking your username on the top-left.</Tooltip
					>
				</h3>

				<pre><code
						>By hash: <a href="/api/w/{$workspaceStore}/jobs/run/h/{script?.hash}"
							>/api/w/{$workspaceStore}/jobs/run/h/{script?.hash}</a
						></code
					></pre>
				<pre><code
						>By path: <a href="/api/w/{$workspaceStore}/jobs/run/p/{script?.path}"
							>/api/w/{$workspaceStore}/jobs/run/p/{script?.path}</a
						></code
					></pre>
				<h3 class="whitespace-nowrap mt-2">
					Endpoint to run this script and get job's result as response
					<Tooltip
						>Send a POST http request with a token as bearer token and the args respecting the
						corresponding jsonschema as payload. To create a permanent token, go to your user
						setting by clicking your username on the top-left.</Tooltip
					>
				</h3>

				<pre><code
						><a href="/api/w/{$workspaceStore}/jobs/run_wait_result/p/{script?.path}"
							>/api/w/{$workspaceStore}/jobs/run_wait_result/p/{script?.path}</a
						></code
					></pre>
			</div>
			<div>
				<h3>
					Previous versions of this hash <Tooltip
						>When you edit a script, a new hash is created and old versions are archived</Tooltip
					>
				</h3>
				<ul>
					{#each script?.parent_hashes ?? [] as p_hash}
						<li><a href="/scripts/get/{p_hash}">{p_hash}</a></li>
					{/each}
				</ul>
			</div>
			<div>
				<div class="grid grid-cols-2 gap-4 pb-1 mb-3 border-b">
					<h3 class="text-gray-700 ">
						Arguments JSON schema <Tooltip
							>The jsonschema defines the constraints that the payload must respect to be compatible
							with the input parameters of this script. The UI form is generated automatically from
							the script jsonschema. See <a href="https://json-schema.org/"
								>jsonschema documentation</a
							></Tooltip
						>
					</h3>
				</div>
				<SchemaViewer schema={script.schema} />
			</div>
			<div>
				<h3 class="text-gray-700 pb-1 mb-3 border-b">Code</h3>
				{#if script.language == 'python3'}
					<Highlight language={python} code={script.content} />
				{:else if script.language == 'deno'}
					<Highlight language={typescript} code={script.content} />
				{/if}
			</div>
			<div>
				<h3 class="text-gray-700 pb-1 mb-3 border-b">Dependencies lock file</h3>
				<pre class="text-xs">{script.lock}</pre>
			</div>
		{/if}
	</div>
</CenteredPage>

<style>
	h3 {
		@apply text-lg mb-2 mt-4 text-gray-600;
	}
</style>
