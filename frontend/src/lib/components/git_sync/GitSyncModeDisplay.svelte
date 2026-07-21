<script lang="ts">
	import { CheckCircle2 } from 'lucide-svelte'
	import type { GitSyncRepository } from './GitSyncContext.svelte'

	// `active` shows a check to signal the direction is live on a saved
	// connection; omit it for pre-save previews (e.g. the setup wizard).
	let {
		mode,
		targetBranch,
		repository,
		active = false
	} = $props<{
		mode?: 'sync' | 'promotion' | null
		targetBranch: string | undefined
		repository?: GitSyncRepository | null
		active?: boolean
	}>()
</script>

<div class="flex items-start gap-1.5 text-2xs text-secondary">
	{#if active}
		<CheckCircle2 size={14} class="mt-px shrink-0 text-green-600" />
	{/if}
	<span>
		{#if mode === 'promotion'}
			On deploy, changes are pushed to a <span class="font-mono">wm_deploy/…</span>
			branch{#if repository?.group_by_folder}
				(grouped by folder){/if}; merging it into {#if targetBranch}<span
					class="font-medium text-primary">{targetBranch}</span
				>{:else}the repo's default branch{/if} promotes the change.
		{:else}
			Matching changes are committed to {#if targetBranch}<span class="font-medium text-primary"
					>{targetBranch}</span
				>{:else}the repo's default branch{/if} on every deploy.
		{/if}
	</span>
</div>
