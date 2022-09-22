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
		scriptToHubUrl,
		copyToClipboard
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
		faGlobe,
		faCodeFork,
		faClipboard
	} from '@fortawesome/free-solid-svg-icons'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { onDestroy } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	let script: Script | undefined
	let topHash: string | undefined
	let can_write = false
	let deploymentInProgress = false
	let intervalId: NodeJS.Timer

	let shareModal: ShareModal

	$: if ($workspaceStore) {
		loadScript($page.params.hash)
	}
	$: webhooks = {
		uuid: {
			hash: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/h/${script?.hash}`,
			path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/p/${script?.path}`
		},
		result: {
			hash: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/h/${script?.hash}`,
			path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/p/${script?.path}`
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
	<div class="flex flex-row flex-wrap justify-between gap-4">
		<div>
			<div class="flex items-center flex-wrap mb-2">
				<h1 class="font-bold text-blue-500 break-all !p-0 mr-2">
					{script?.path ?? 'Loading...'}
				</h1>
				<div class="flex items-center gap-2">
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
					{#if deploymentInProgress}
						<Badge
							color="yellow"
							icon={{ data: faSpinner, position: 'right', class: 'animate-spin' }}
						>
							Deployment in progress
						</Badge>
					{/if}
				</div>
			</div>
			<p class="mb-2">
				<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
				<span class="text-sm text-gray-500">
					{#if script}
						Edited {displayDaysAgo(script.created_at || '')} by {script.created_by || 'unknown'}
					{/if}
				</span>
			</p>
		</div>

		{#if script}
			<div class="flex items-start flex-wrap gap-1">
				<a
					class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
					href="/scripts/run/{script.hash}"
				>
					<div class="inline-flex items-center justify-center">
						<Icon class="text-blue-500" data={faPlay} scale={0.5} />
						<span class="pl-1">Run</span>
					</div>
				</a>
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
				{#if !topHash}
					<a
						class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
						href="/scripts/add?template={script.path}"
					>
						<div class="inline-flex items-center justify-center">
							<Icon class="text-blue-500" data={faCodeFork} scale={0.5} />
							<span class="pl-1">Use as template/Fork</span>
						</div>
					</a>
				{/if}
				<a
					class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
					href="/runs/{script.path}"
				>
					<div class="inline-flex items-center justify-center">
						<Icon class="text-blue-500" data={faList} scale={0.5} />
						<span class="pl-1">View runs</span>
					</div>
				</a>
				<a
					target="_blank"
					class="inline-flex items-center default-button bg-transparent hover:bg-blue-500 text-blue-700 font-normal hover:text-white py-0 px-1 border-blue-500 hover:border-transparent rounded"
					href={scriptToHubUrl(
						script.content,
						script.summary,
						script.description ?? '',
						script.kind
					).toString()}
				>
					<div class="inline-flex items-center justify-center">
						<Icon class="text-blue-500" data={faGlobe} scale={0.5} />
						<span class="pl-1">Publish to Hub</span>
					</div>
				</a>
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
			</div>
		{/if}
	</div>

	<ShareModal bind:this={shareModal} kind="script" path={script?.path ?? ''} />

	<div class="flex flex-col gap-8 max-w-7xl pb-2">
		{#if script === undefined}
			<p>loading</p>
		{:else}
			<div>
				<h2 class="font-bold mt-8 mb-2">{script.summary}</h2>
				<div class="prose">
					<SvelteMarkdown source={defaultIfEmptyString(script.description, 'No description')} />
				</div>
			</div>

			{#if script.lock_error_logs || topHash || script.archived || script.deleted}
				<div class="flex flex-col gap-2">
					{#if script.lock_error_logs}
						<div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4" role="alert">
							<p class="font-bold">Error deploying this script</p>
							<p>This script has not been deployed successfully because of the following errors:</p>
							<pre class="w-full text-xs mt-2 whitespace-pre-wrap">{script.lock_error_logs}</pre>
						</div>
					{/if}
					{#if topHash}
						<div
							class="bg-orange-100 border-l-4 border-orange-500 text-orange-700 p-4"
							role="alert"
						>
							<p class="font-bold">Not HEAD</p>
							<p>
								This hash is not HEAD (latest non-archived version at this path) :
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
				</div>
			{/if}

			<div class="flex flex-col lg:flex-row gap-4">
				<div class="lg:w-1/2">
					<h3 class="text-lg mb-1 font-bold text-gray-600">Webhooks</h3>
					<div class="border rounded-sm shadow-sm p-4">
						<Tabs selected="uuid">
							<Tab value="uuid">UUID</Tab>
							<Tab value="result">Result</Tab>
							<svelte:fragment slot="content">
								{#each Object.keys(webhooks) as key}
									<TabContent value={key}>
										<ul>
											{#each Object.keys(webhooks[key]) as type}
												{@const url = webhooks[key][type]}
												<li class="flex justify-between items-center mt-2">
													<a
														href={'//' + url}
														class="whitespace-nowrap text-clip overflow-hidden mr-1"
													>
														{url}
													</a>
													<div class="flex">
														<Badge color="dark-gray">{type}</Badge>
														<button
															on:click|preventDefault={() => copyToClipboard(url)}
															class="flex items-center bg-blue-600 text-white rounded-md px-2 ml-2"
														>
															<Icon data={faClipboard} />
															<span class="ml-1">Copy</span>
														</button>
													</div>
												</li>
											{/each}
										</ul>
									</TabContent>
								{/each}
							</svelte:fragment>
						</Tabs>
					</div>
				</div>
				<div class="lg:w-1/2">
					<h3 class="text-lg mb-1 font-bold text-gray-600">Versions</h3>
					<div class="border rounded-sm shadow-sm p-4">
						<h4 class="font-bold text-gray-500">Current</h4>
						<div class="mt-1">
							{script?.hash}
						</div>
						<h4 class="font-bold text-gray-500 mt-2">Previous</h4>
						{#if script?.parent_hashes?.length}
							<ul class="max-h-20 overflow-y-auto">
								{#each script.parent_hashes as hash}
									<li class="mt-1">
										<a href="/scripts/get/{hash}">{hash}</a>
									</li>
								{/each}
							</ul>
						{:else}
							<p class="text-sm text-gray-500">There are no previous versions</p>
						{/if}
					</div>
				</div>
			</div>
			<div>
				<h3 class="text-lg mb-1 font-bold text-gray-600">
					Arguments JSON schema
					<Tooltip>
						The jsonschema defines the constraints that the payload must respect to be compatible
						with the input parameters of this script. The UI form is generated automatically from
						the script jsonschema. See
						<a href="https://json-schema.org/"> jsonschema documentation </a>
					</Tooltip>
				</h3>
				<SchemaViewer schema={script.schema} />
			</div>
			<div>
				<h3 class="text-lg mb-1 font-bold text-gray-600">Code</h3>
				<HighlightCode language={script.language} code={script.content} />
			</div>
			<div>
				<h3 class="text-lg mb-1 font-bold text-gray-600">Dependencies lock file</h3>
				{#if script?.lock}
					<pre class="text-xs">{script.lock}</pre>
				{:else}
					<p class="text-sm text-gray-500">There is no lock file for this script</p>
				{/if}
			</div>
		{/if}
	</div>
</CenteredPage>
