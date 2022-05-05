<script lang="ts">
	import RadioButton from './RadioButton.svelte';
	import ResourceTypePicker from './ResourceTypePicker.svelte';

	export let format: string | undefined;

	let kind: 'resource' | 'none' = format?.startsWith('resource') ? 'resource' : 'none';

	let resource: string | undefined = format?.startsWith('resource-')
		? format.substring('resource-'.length)
		: undefined;

	$: format =
		kind == 'resource' ? (resource != undefined ? `resource-${resource}` : 'resource') : undefined;
</script>

<RadioButton
	label="Kind"
	small={true}
	options={[
		['None', 'none'],
		['Resource Type', 'resource']
	]}
	bind:value={kind}
/>

{#if kind == 'resource'}
	<div class="mt-1" />
	<ResourceTypePicker bind:value={resource} />
{/if}
