<script lang="ts">
	import { page } from '$app/stores'
	import { JobService, ScriptService, type Script } from '$lib/gen'
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
		faGlobe,
		faCodeFork,
		faClipboard,
		faArrowLeft,
		faChevronUp,
		faChevronDown
	} from '@fortawesome/free-solid-svg-icons'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { onDestroy } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import {
		Badge,
		Tabs,
		Tab,
		TabContent,
		Button,
		Alert,
		ButtonPopup,
		ButtonPopupItem
	} from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import Icon from 'svelte-awesome'
	import RunForm from '$lib/components/RunForm.svelte'
	import { goto } from '$app/navigation'
	import Popover from '$lib/components/Popover.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'

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

	let isValid = true
	let runForm: RunForm | undefined

	let runLoading = false
	async function runScript(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		try {
			runLoading = true
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
			let run = await JobService.runScriptByHash({
				workspace: $workspaceStore!,
				hash: script?.hash ?? '',
				requestBody: args,
				scheduledFor,
				invisibleToOwner
			})
			await goto('/run/' + run + '?workspace=' + $workspaceStore)
		} catch (err) {
			runLoading = false
			sendUserToast(`Could not create job: ${err.body}`, true)
		}
	}
	let scheduleEditor: ScheduleEditor
	let webhookElem: HTMLHeadingElement

	let viewWebhookCommand = false

	let args = undefined
	$: curlCommand = `curl -H 'Content-Type: application/json' -H "Authorization: Bearer $TOKEN" -X POST -d '${JSON.stringify(
		args
	)}' ${$page.url.protocol}//${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/p/${
		script?.path
	}`
	let moveDrawer: MoveDrawer
</script>

<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/scripts/get/' + e.detail)
		loadScript($page.params.hash)
	}}
/>

<ScheduleEditor bind:this={scheduleEditor} />

{#if script}
	<CenteredPage>
		<Skeleton {loading} layout={[[{ h: 1.5, w: 40 }], 1, [{ h: 1, w: 30 }]]} />

		<div class="prose-sm mx-auto mt-6">
			<div class="flex flex-row w-full justify-between item-center">
				<h1 class="mb-1 truncate">{defaultIfEmptyString(script.summary, script.path)}</h1>

				<div class="flex flex-row-reverse gap-2 h-full">
					<Button
						href={`/scripts/run/${script.hash}`}
						color="blue"
						size="md"
						startIcon={{ icon: faPlay }}
					>
						Run
					</Button>
					{#if !$userStore?.operator}
						<Button
							href={`/scripts/edit/${script.hash}?step=2`}
							color="blue"
							size="md"
							startIcon={{ icon: faEdit }}
							disabled={!can_write}
						>
							Edit
						</Button>
						{#if !topHash}
							<Button
								href={`/scripts/add?template=${script.path}`}
								variant="border"
								size="md"
								startIcon={{ icon: faCodeFork }}
							>
								Fork
							</Button>
						{/if}
					{/if}
					<Button
						href={`/runs/${script.path}`}
						size="md"
						startIcon={{ icon: faList }}
						color="light"
						variant="border"
					>
						View Runs
					</Button>
				</div>
			</div>

			{#if !emptyString(script.summary)}
				<span class="text-lg font-semibold">{script.path}</span>
			{/if}

			<div class="flex flex-row gap-x-2 flex-wrap items-center mt-4">
				<span class="text-sm text-gray-600">
					Edited {displayDaysAgo(script.created_at || '')} by {script.created_by || 'unknown'}
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
				{#if deploymentInProgress}
					<Badge color="yellow">
						<Loader2 size={12} class="inline animate-spin mr-1" />
						Deployment in progress
					</Badge>
				{/if}
				<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
			</div>

			<div class="flex gap-2 flex-wrap mt-4">
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
					on:click={() => shareModal.openDrawer(script?.path ?? '', 'script')}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faShare }}
					disabled={!can_write}
				>
					Share
				</Button>
				<Button
					on:click={() => scheduleEditor?.openNew(false, script?.path ?? '')}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faCalendar }}
				>
					Schedule
				</Button>
				<Button
					on:click={() => moveDrawer.openDrawer(script?.path ?? '', script?.summary, 'script')}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faEdit }}
				>
					Move/Rename
				</Button>
				<Button
					color="dark"
					variant="border"
					size="xs"
					on:click={() => webhookElem.scrollIntoView()}>Webhooks</Button
				>
				{#if Array.isArray(script.parent_hashes) && script.parent_hashes.length > 0}
					<ButtonPopup
						color="dark"
						variant="contained"
						size="xs"
						startIcon={{ icon: faArrowLeft }}
						href="/scripts/get/{script.parent_hashes[0]}"
					>
						<svelte:fragment slot="main">
							Previous version ({script.parent_hashes.length})
						</svelte:fragment>

						{#each script.parent_hashes as hash}
							<ButtonPopupItem href="/scripts/get/{hash}" btnClasses="!m-0">
								{hash}
							</ButtonPopupItem>
						{/each}
					</ButtonPopup>
				{/if}
			</div>

			{#if script.lock_error_logs || topHash || script.archived || script.deleted}
				<div class="flex flex-col gap-2 my-2">
					{#if script.lock_error_logs}
						<div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4" role="alert">
							<p class="font-bold">Error deploying this script</p>
							<p>This script has not been deployed successfully because of the following errors:</p>
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

			<div class="mt-6 grid grid-cols-1 sm:grid-cols-3 gap-6">
				<div class="col-span-2 box">
					<RunForm
						loading={runLoading}
						autofocus
						detailed={false}
						bind:isValid
						bind:this={runForm}
						runnable={script}
						runAction={runScript}
						bind:args
					/>
				</div>
				{#if !emptyString(script.description)}
					<div class="box">
						{defaultIfEmptyString(script.description, 'No description')}
					</div>
				{/if}
			</div>

			<div class="mt-8">
				<Skeleton {loading} layout={[[20]]} />

				<Tabs selected="code">
					<Tab value="code">Code</Tab>
					<Tab value="dependencies">Dependencies lock file</Tab>
					<Tab value="arguments"
						><span class="inline-flex items-center gap-1">
							Arguments JSON Schema
							<Tooltip>
								The jsonschema defines the constraints that the payload must respect to be
								compatible with the input parameters of this script. The UI form is generated
								automatically from the script jsonschema. See
								<a href="https://json-schema.org/" class="text-blue-500">
									jsonschema documentation
								</a>
							</Tooltip>
						</span>
					</Tab>
					<svelte:fragment slot="content">
						<TabContent value="code">
							<div class="border rounded-sm mt-2">
								<HighlightCode language={script.language} code={script.content} />
							</div>
						</TabContent>
						<TabContent value="dependencies">
							<div class="border rounded-sm mt-2">
								{#if script?.lock}
									<pre>{script.lock}</pre>
								{:else}
									<p>There is no lock file for this script</p>
								{/if}
							</div>
						</TabContent>
						<TabContent value="arguments">
							<div class="max-w-2xl">
								<SchemaViewer schema={script.schema} />
							</div>
						</TabContent>
					</svelte:fragment>
				</Tabs>
			</div>

			<div class="max-w-2xl mt-12">
				<h3 bind:this={webhookElem} id="webhooks">
					Webhooks
					<Tooltip>
						Pass the input as a json payload, the token as a Bearer token or as query arg
						`?token=XXX` and pass as header: 'Content-Type: application/json'
						<a href="https://docs.windmill.dev/docs/getting_started/webhooks" class="text-blue-500">
							See docs
						</a>
					</Tooltip>
				</h3>
				<Skeleton {loading} layout={[[8.5]]} />
				<Tabs selected="uuid">
					<Tab value="uuid">UUID</Tab>
					<Tab value="result">Result</Tab>
					<svelte:fragment slot="content">
						{#each Object.keys(webhooks) as key}
							<TabContent value={key}>
								<ul>
									{#each Object.keys(webhooks[key]) as type}
										{@const url = webhooks[key][type]}
										{@const href = $page.url.protocol + '//' + url}
										<li class="flex justify-between items-center mt-2">
											<a
												on:click={(e) => {
													e.preventDefault()
													copyToClipboard(href)
												}}
												{href}
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
						<Button
							color="light"
							size="sm"
							endIcon={{ icon: viewWebhookCommand ? faChevronUp : faChevronDown }}
							on:click={() => (viewWebhookCommand = !viewWebhookCommand)}
						>
							See example curl command
						</Button>
						{#if viewWebhookCommand}
							<div transition:slide|local class="px-4">
								<pre class="bg-gray-700 text-gray-100 p-2  font-mono text-sm whitespace-pre-wrap"
									>{curlCommand} <span
										on:click={() => copyToClipboard(curlCommand)}
										class="cursor-pointer ml-2"><Icon data={faClipboard} /></span
									></pre
								>
							</div>
						{/if}
					</svelte:fragment>
				</Tabs>
			</div>
			<div class="mt-20">
				{#if can_write}
					<h3>Danger zone</h3>
					<div class="flex gap-2">
						<Popover>
							<Button
								size="xs"
								on:click={() => {
									script?.hash && deleteScript(script.hash)
								}}
								color="red"
								variant="contained"
								startIcon={{ icon: faTrash }}
								disabled={!($superadmin || ($userStore?.is_admin ?? false))}
							>
								Delete
							</Button>
							<span slot="text">require to be admin</span>
						</Popover>
						<Button
							size="xs"
							on:click={() => {
								script?.hash && archiveScript(script.hash)
							}}
							color="red"
							variant="border"
							startIcon={{ icon: faArchive }}
						>
							Archive
						</Button>
					</div>
				{/if}
			</div>
		</div>
	</CenteredPage>
{/if}

<UserSettings bind:this={userSettings} />

<ShareModal bind:this={shareModal} />
