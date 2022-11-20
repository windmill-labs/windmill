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
		copyToClipboard,
		emptyString
	} from '$lib/utils'
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
	import { Badge, Tabs, Tab, TabContent, Button, ActionRow, Alert } from '$lib/components/common'
	import Skeleton from '../../../lib/components/common/skeleton/Skeleton.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import Icon from 'svelte-awesome'

	let userSettings: UserSettings
	let script: Script | undefined
	let topHash: string | undefined
	let can_write = false
	let deploymentInProgress = false
	let intervalId: NodeJS.Timer

	let shareModal: ShareModal

	$: loading = !script
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

<UserSettings bind:this={userSettings} />

<Skeleton {loading} class="!px-4 sm:!px-6 md:!px-8 !max-w-6xl" layout={[0.75, [2, 0, 2], 1]} />
<div class="h-screen relative overflow-auto">
	{#if script}
		<ActionRow applyPageWidth stickToTop>
			<svelte:fragment slot="left">
				<Dropdown
					left={false}
					dropdownItems={[
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
					]}><span class="text-red-500">delete...</span></Dropdown
				>
				<Button
					disabled={deploymentInProgress}
					target="_blank"
					href={scriptToHubUrl(
						script.content,
						script.summary,
						script.description ?? '',
						script.kind,
						script.language,
						script.schema,
						script.language == 'deno' ? '' : script.lock
					).toString()}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faGlobe }}
				>
					Publish to Hub
				</Button>
				<Button
					on:click={() => shareModal.openDrawer()}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faShare }}
					disabled={!can_write}
				>
					Share
				</Button>
				<Button
					href="/schedule/add?path={script.path}"
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faCalendar }}
				>
					Schedule
				</Button>
			</svelte:fragment>
			<svelte:fragment slot="right">
				<Button
					href={`/scripts/run/${script.hash}`}
					color="blue"
					size="xs"
					startIcon={{ icon: faPlay }}
				>
					Run
				</Button>
				<Button
					href={`/scripts/edit/${script.hash}?step=2`}
					color="blue"
					size="xs"
					startIcon={{ icon: faEdit }}
					disabled={!can_write}
				>
					Edit
				</Button>
				{#if !topHash}
					<Button
						href={`/scripts/add?template=${script.path}`}
						color="blue"
						size="xs"
						startIcon={{ icon: faCodeFork }}
					>
						Use as template/Fork
					</Button>
				{/if}
				<Button href={`/runs/${script.path}`} size="xs" startIcon={{ icon: faList }}>
					View runs
				</Button>
			</svelte:fragment>
		</ActionRow>
	{/if}
	<CenteredPage>
		<Skeleton {loading} layout={[[{ h: 1.5, w: 40 }], 1, [{ h: 1, w: 30 }]]} />
		{#if script}
			<div class="flex flex-row flex-wrap justify-between gap-4">
				<div>
					<div class="flex flex-col mb-2">
						<h1 class="break-words py-2 mr-2">
							{defaultIfEmptyString(script.summary, script.path)}
						</h1>
						{#if !emptyString(script.summary)}
							<h2 class="font-bold pb-4">{script.path}</h2>
						{/if}

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
							<a href="#webhooks">
								<Badge color="dark-blue">Webhooks</Badge>
							</a>
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
							<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
						</div>
					</div>
				</div>
			</div>
		{/if}

		<ShareModal bind:this={shareModal} kind="script" path={script?.path ?? ''} />

		<div class="flex flex-col gap-8 max-w-7xl pt-4 pb-2">
			<Skeleton {loading} layout={[[3], 1]} />

			{#if script}
				<div class="flex flex-col lg:flex-row gap-4 mt-4">
					<div class="lg:w-1/2">
						<h3 class="text-lg mb-1 font-bold text-gray-600">Description</h3>

						<div class="prose text-sm box">
							<SvelteMarkdown source={defaultIfEmptyString(script.description, 'No description')} />
						</div>
					</div>
					<div class="lg:w-1/2">
						<h3 class="text-lg mb-1 font-bold text-gray-600">Versions</h3>
						<Skeleton {loading} layout={[[8.5]]} />
						{#if script}
							<div class="box">
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
						{/if}
					</div>
				</div>

				{#if script.lock_error_logs || topHash || script.archived || script.deleted}
					<div class="flex flex-col gap-2">
						{#if script.lock_error_logs}
							<div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4" role="alert">
								<p class="font-bold">Error deploying this script</p>
								<p
									>This script has not been deployed successfully because of the following errors:</p
								>
								<pre class="w-full text-xs mt-2 whitespace-pre-wrap">{script.lock_error_logs}</pre>
							</div>
						{/if}
						{#if topHash}
							<Alert type="warning" title="Not HEAD">
								This hash is not HEAD (latest non-archived version at this path) :
								<a href="/scripts/get/{topHash}">Go to the HEAD of this path</a>
							</Alert>
						{/if}
						{#if script.archived && !topHash}
							<Alert type="error" title="Archived">This version was archived</Alert>
						{/if}
						{#if script.deleted}
							<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4" role="alert">
								<p class="font-bold">Deleted</p>
								<p>The content of this script was deleted (by an admin, no less)</p>
							</div>
						{/if}
					</div>
				{/if}
			{/if}

			<div class="mt-4">
				<h2 class="text-lg mb-1 font-bold text-gray-600">
					Arguments JSON schema
					<Tooltip>
						The jsonschema defines the constraints that the payload must respect to be compatible
						with the input parameters of this script. The UI form is generated automatically from
						the script jsonschema. See
						<a href="https://json-schema.org/"> jsonschema documentation </a>
					</Tooltip>
				</h2>
				<Skeleton {loading} layout={[[15]]} />
				{#if script}
					<div class="box mt-2">
						<SchemaViewer schema={script.schema} />
					</div>
				{/if}
			</div>
			<div>
				<h2 class="text-lg mb-1 mt-4 font-bold text-gray-600 border-b">Code</h2>
				<Skeleton {loading} layout={[[20]]} />
				{#if script}
					<HighlightCode language={script.language} code={script.content} />
				{/if}
			</div>
		</div>

		<h3 id="webhooks" class="text-lg mb-1 mt-8 font-bold text-gray-600"
			>Webhooks<Tooltip
				>To trigger this script with a webhook, do a POST request to the endpoints below. Scripts
				are not public and can only be run by users with at least view rights on them. You will need
				to pass a bearer token to authentify as a user. You can either pass it as a Bearer token or
				as query arg `?token=XXX`. <a href="https://docs.windmill.dev/docs/getting_started/webhooks"
					>See docs</a
				></Tooltip
			></h3
		>
		<Skeleton {loading} layout={[[8.5]]} />
		{#if script}
			<div class="box max-w-3xl">
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
												on:click={(e) => {
													e.preventDefault()
													copyToClipboard(url)
												}}
												href={$page.url.protocol + '//' + url}
												class="whitespace-nowrap text-ellipsis overflow-hidden mr-1"
											>
												{url}
												<span class="text-gray-700 ml-2">
													<Icon data={faClipboard} />
												</span>
											</a>
											<Badge color="dark-gray" capitalize>
												{type}
											</Badge>
										</li>
									{/each}
								</ul>
								<div class="flex flex-row-reverse mt-2">
									<Button size="xs" on:click={userSettings.openDrawer}>Create token</Button>
								</div>
							</TabContent>
						{/each}
					</svelte:fragment>
				</Tabs>
			</div>
		{/if}
		<div>
			<h2 class="text-lg mb-1 font-bold text-gray-600 border-b mt-8">Dependencies lock file</h2>
			<Skeleton {loading} layout={[[5]]} />
			{#if script}
				{#if script?.lock}
					<pre class="text-xs overflow-auto">{script.lock}</pre>
				{:else}
					<p class="text-sm text-gray-500">There is no lock file for this script</p>
				{/if}
			{/if}
		</div>
	</CenteredPage>
</div>
