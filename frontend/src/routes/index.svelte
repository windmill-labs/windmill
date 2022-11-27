<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import JobDetail from '$lib/components/jobs/JobDetail.svelte'
	import FlowGettingStarted from '$lib/components/landing/FlowGettingStarted.svelte'
	import FlowLandingBox from '$lib/components/landing/FlowLandingBox.svelte'
	import RessourceGettingStarted from '$lib/components/landing/RessourceGettingStarted.svelte'
	import ScriptBox from '$lib/components/landing/ScriptBox.svelte'
	import ScriptGettingStarted from '$lib/components/landing/ScriptGettingStarted.svelte'
	import { FlowService, Job, JobService, Script, ScriptService, type Flow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { Alert, Skeleton } from '$lib/components/common'

	let scripts: Script[] = []
	let flows: Flow[] = []
	let jobs: Job[] = []
	let loading = {
		scripts: true,
		flows: true,
		jobs: true
	}

	async function loadScripts() {
		scripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			perPage: 3
		})
		loading.scripts = false
	}
	async function loadFlows() {
		flows = await FlowService.listFlows({
			workspace: $workspaceStore!,
			perPage: 3
		})
		loading.flows = false
	}

	async function loadJobs() {
		jobs = await JobService.listJobs({
			workspace: $workspaceStore!,
			success: true,
			createdBy: $userStore?.username,
			jobKinds: 'flow,script'
		})
		loading.jobs = false
	}

	$: {
		if ($userStore && $workspaceStore) {
			loadScripts()
			loadFlows()
			loadJobs()
		}
	}

	const resources = []
</script>

<CenteredPage>
	<h1 class="flex items-center min-h-[48px] font-black my-4">Home</h1>
	<div class="space-y-8">
		{#if $workspaceStore == 'demo'}
			<Alert title="Demo workspace">The demo workspace shared in which all users get invited.</Alert
			>
		{:else if $workspaceStore == 'starter'}
			<Alert title="Stater workspace"
				>The starter workspace has all its elements (variables, resources, scripts, flows) shared
				across all other workspaces. Useful to seed workspace with common elements within your
				organization.</Alert
			>
		{/if}
		<div>
			<h2 class="border-b mb-4 py-2">
				<span class="text-black-gradient">Scripts</span>
			</h2>
			<ScriptGettingStarted />

			<div class="mt-6 mb-2 text-md font-bold text-gray-900 ">Latest scripts:</div>
			<Skeleton loading={loading.scripts} layout={[0.5, [12.25, 12.25, 12.25]]} />
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 my-4">
				{#each scripts as script}
					<ScriptBox {script} />
				{/each}
				<a
					href="/scripts"
					class="text-sm font-extrabold text-gray-700 hover:underline inline-flex items-center"
				>
					All scripts
					<svg
						class="w-4 h-4 ml-2"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
						xmlns="http://www.w3.org/2000/svg"
						><path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M17 8l4 4m0 0l-4 4m4-4H3"
						/>
					</svg>
				</a>
			</div>
		</div>
		<div>
			<h2 class="border-b mb-4 py-2">
				<span class="text-black-gradient">Flows</span>
			</h2>
			<FlowGettingStarted />
			<div class="mt-6 mb-2 text-md font-bold text-gray-900 ">Latest flows:</div>

			<Skeleton loading={loading.flows} layout={[1, [13.5, 13.5, 13.5]]} />
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 my-4">
				{#each flows as flow}
					<FlowLandingBox {flow} />
				{/each}
				<a
					href="/flows"
					class="text-sm font-extrabold text-gray-700 hover:underline inline-flex items-center"
				>
					All flows
					<svg
						class="w-4 h-4 ml-2"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
						xmlns="http://www.w3.org/2000/svg"
						><path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M17 8l4 4m0 0l-4 4m4-4H3"
						/>
					</svg>
				</a>
			</div>
		</div>
		<div>
			<h2 class="border-b mb-4 py-2">
				<span class="text-black-gradient">Resources</span>
			</h2>

			{#if resources.length === 0}
				<RessourceGettingStarted />
			{/if}
		</div>
		<div>
			<h2 class="border-b mb-4 py-2">
				<span class="text-black-gradient">Runs</span>
			</h2>

			<div class="grid grid-cols-1 gap-4 my-4">
				<Skeleton loading={loading.jobs} layout={[[6], 1, [6], 1, [6]]} />
				{#each jobs.splice(0, 3) as job}
					<JobDetail {job} />
				{/each}
				<a
					href="/runs"
					class="text-sm font-extrabold text-gray-700 hover:underline inline-flex items-center"
				>
					All runs
					<svg
						class="w-4 h-4 ml-2"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
						xmlns="http://www.w3.org/2000/svg"
						><path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M17 8l4 4m0 0l-4 4m4-4H3"
						/>
					</svg>
				</a>
			</div>
		</div>
	</div>
</CenteredPage>
