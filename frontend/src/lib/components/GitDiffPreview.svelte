<script lang="ts">
	type DiffResult = {
		added: string[]
		deleted: string[]
		modified: string[]
		repoWmillYaml?: string
		yamlModified?: boolean
	}

	let { previewResult } = $props<{
		previewResult: DiffResult | undefined
	}>()
</script>

<div class="border rounded p-2 text-xs max-h-40 overflow-y-auto bg-surface-secondary">
	<div class="font-semibold text-[11px] mb-1 text-tertiary">Preview of changes:</div>
	{#if !previewResult?.added?.length && !previewResult?.deleted?.length && !previewResult?.modified?.length && !previewResult?.yamlModified}
		<div class="mt-2 text-tertiary">No changes found! The workspace is up to date.</div>
	{:else}
		{#if previewResult?.yamlModified}
			<div class="mt-2">
				<div class="text-yellow-600">Modified:</div>
				<ul class="list-disc list-inside">
					<li>wmill.yaml (Git sync settings)</li>
				</ul>
			</div>
		{/if}
		{#if previewResult?.added?.length}
			<div class="mt-2">
				<div class="text-green-600">Added:</div>
				<ul class="list-disc list-inside">
					{#each previewResult.added as file}
						<li>
							{file}{!file.includes('.') ? ' (dir)' : ''}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if previewResult?.deleted?.length}
			<div class="mt-2">
				<div class="text-red-600">Deleted:</div>
				<ul class="list-disc list-inside">
					{#each previewResult.deleted as file}
						<li>
							{file}{!file.includes('.') ? ' (dir)' : ''}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if previewResult?.modified?.length}
			<div class="mt-2">
				<div class="text-yellow-600">Modified:</div>
				<ul class="list-disc list-inside">
					{#each previewResult.modified as file}
						<li>
							{file}{!file.includes('.') ? ' (dir)' : ''}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	{/if}
</div>
