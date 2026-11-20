<script lang="ts" module>
	import { lazyModule } from '$lib/lazyModule.svelte'

	const createWorkspaceModule = lazyModule(
		() => import('$lib/components/workspaceSettings/CreateWorkspaceInner.svelte')
	)
</script>

<script lang="ts">
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import WorkspaceScopeTrigger from '$lib/components/WorkspaceScopeTrigger.svelte'
	import { Badge } from '$lib/components/common'
	import type { Session } from './sessionState.svelte'
	import type { useForkStaging } from './forkStaging.svelte'

	let { session, staging }: { session: Session; staging: ReturnType<typeof useForkStaging> } =
		$props()

	const staged = $derived(session.pending_fork)

	function toggle() {
		if (staged) staging.unstage()
		else void staging.stage()
	}

	let settingsOpen = $state(false)
	function openSettings() {
		void createWorkspaceModule.load()
		settingsOpen = true
	}
</script>

{#if staging.available}
	<div class="flex items-center gap-1.5 min-w-0 text-2xs text-secondary">
		<label class="flex items-center gap-1.5 shrink-0 cursor-pointer">
			<Checkbox checked={!!staged} onChange={toggle} />
			<span>Work in fork</span>
		</label>
		{#if !staged && session.direct_in && session.direct_in === staging.workspace}
			<Badge
				color="orange"
				small
				title="You chose to work in {session.direct_in} itself. Check Work in fork to build in a fork instead."
			>
				Changes go directly to {session.direct_in}
			</Badge>
		{/if}
		{#if staged}
			<WorkspaceScopeTrigger
				workspaceId={staged.parent_workspace_id}
				pendingFork={staged}
				color={staged.settings?.color}
				class="min-w-0"
				compact
				onClick={openSettings}
			/>
		{/if}
	</div>
{/if}

<Modal2
	title="Forking {staging.workspace}"
	target="#content"
	fixedHeight="adaptive"
	fixedWidth="sm"
	contentClasses="flex-col"
	bind:isOpen={settingsOpen}
>
	{#if settingsOpen && staged && createWorkspaceModule.current}
		{@const CreateWorkspaceInner = createWorkspaceModule.current.default}
		<CreateWorkspaceInner
			isFork
			inModal
			stage={{ fork: staged, onStage: staging.save }}
			onFinish={() => (settingsOpen = false)}
		/>
	{/if}
</Modal2>
