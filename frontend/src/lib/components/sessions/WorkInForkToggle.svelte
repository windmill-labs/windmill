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
	import { Badge, Button } from '$lib/components/common'
	import { X } from 'lucide-svelte'
	import type { Session } from './sessionState.svelte'
	import type { useForkStaging } from './forkStaging.svelte'

	let {
		session,
		staging,
		onLeaveDirect
	}: {
		session: Session
		staging: ReturnType<typeof useForkStaging>
		onLeaveDirect: () => void
	} = $props()

	const staged = $derived(session.pending_fork)
	// Shown even where forking is unavailable: it is then the only way back out of direct work.
	const direct = $derived(
		!staged && !!session.direct_in && session.direct_in === staging.workspace
			? session.direct_in
			: undefined
	)

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

{#if staging.available || direct}
	<div class="flex items-center gap-1.5 min-w-0 text-2xs text-secondary">
		{#if staging.available}
			<label class="flex items-center gap-1.5 shrink-0 cursor-pointer">
				<Checkbox checked={!!staged} onChange={toggle} />
				<span>Work in fork</span>
			</label>
		{/if}
		{#if direct}
			<Badge color="orange" small title="You chose to work in {direct} itself.">
				Changes go directly to {direct}
			</Badge>
			<Button
				variant="subtle"
				unifiedSize="2xs"
				iconOnly
				startIcon={{ icon: X }}
				title="Stop working in {direct} directly"
				onClick={onLeaveDirect}
			/>
		{/if}
		{#if staged}
			<WorkspaceScopeTrigger
				workspaceId={staged.parent_workspace_id}
				pendingFork={staged}
				color={staged.settings?.color}
				class="min-w-0"
				compact
				stagedDev={staged.settings?.is_dev_workspace
					? { label: staged.settings.dev_workspace_label }
					: undefined}
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
