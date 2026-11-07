<script lang="ts">
	import { Button } from '$lib/components/common'
	import PierreMergeConflict from '$lib/components/pierre/PierreMergeConflict.svelte'
	import { MERGE_CONFLICT_SAMPLE } from './cases'

	let resolved: string | undefined = $state()
	let resetKey = $state(0)
</script>

<div class="grid gap-4 xl:grid-cols-2">
	<section class="flex min-w-0 flex-col gap-2">
		<div class="flex items-center gap-2">
			<h2 class="text-sm font-semibold">Pierre: merge conflicts (beta)</h2>
			<Button
				unifiedSize="xs"
				variant="subtle"
				onclick={() => {
					resolved = undefined
					resetKey++
				}}>Reset</Button
			>
		</div>
		<p class="text-2xs text-hint">
			Each conflict offers current, incoming or both. Windmill has no conflict UI today: a fork
			merge or a git sync pull with diverged items would be the place for it.
		</p>
		<div class="h-[480px] overflow-auto border">
			{#key resetKey}
				<PierreMergeConflict
					contents={MERGE_CONFLICT_SAMPLE}
					filename="customer.ts"
					lang="typescript"
					onResolved={(c) => (resolved = c)}
				/>
			{/key}
		</div>
	</section>
	<section class="flex min-w-0 flex-col gap-2">
		<h2 class="text-sm font-semibold">Resolved output</h2>
		<pre class="h-[510px] overflow-auto border p-2 text-2xs"
			>{resolved ?? 'Resolve a conflict to see the result'}</pre
		>
	</section>
</div>
