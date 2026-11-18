<script lang="ts">
	import WorkerTagSelect from './WorkerTagSelect.svelte'

	interface Props {
		/** Tag override; undefined runs the jobs on `defaultTag`. */
		tag: string | undefined
		/** Tag the jobs carry with no override, i.e. the database language's native one. */
		defaultTag: string | undefined
		/** Workspace the custom tags are read from; defaults to the navigation one. */
		workspace?: string
		class?: string
	}

	let {
		tag = $bindable(),
		defaultTag,
		workspace = undefined,
		class: className = ''
	}: Props = $props()
</script>

<div class={'flex flex-col gap-1 ' + className}>
	<span class="text-xs font-semibold text-secondary">Worker tag</span>
	<WorkerTagSelect bind:tag noLabel nullTag={defaultTag} workspaceId={workspace} size="sm" />
	<span class="text-xs text-tertiary">
		Queries of this database run as jobs tagged <b>{defaultTag}</b>. Pick a custom tag to run them
		on a specific worker group instead, e.g. the only one that can reach this database.
	</span>
</div>
