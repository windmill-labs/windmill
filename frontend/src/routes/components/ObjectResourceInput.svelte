<script lang="ts">
	import { ResourceService } from '../../gen';

	import ResourcePicker from './ResourcePicker.svelte';
	import { workspaceStore } from '../../stores';
	import SchemaForm from './SchemaForm.svelte';
	import RadioButton from './RadioButton.svelte';

	export let format: string;
	export let value: any;

	function isString(value: any) {
		return typeof value === 'string' || value instanceof String;
	}

	let path: string =
		isString(value) && value.length >= '$res:'.length ? value.substr('$res:'.length) : undefined;
	let args: Record<string, any> = {};

	if (!isString(value) && value) {
		console.log(value);
		args = value;
	}

	let schema: any | undefined = undefined;
	let isValid = true;
	let resourceTypeName: string = '';

	async function loadSchema(format: string) {
		resourceTypeName = format.substring('resource-'.length);
		schema = (
			await ResourceService.getResourceType({ workspace: $workspaceStore!, path: resourceTypeName })
		).schema;
	}

	let option: 'resource' | 'raw' = isString(value) || value == undefined ? 'resource' : 'raw';

	$: {
		if (option == 'resource') {
			value = `$res:${path}`;
		} else {
			value = args;
		}
	}
	$: format.startsWith('resource-') && loadSchema(format);
</script>

<div class="max-w-lg">
	<RadioButton
		options={[
			[`Resource (${resourceTypeName})`, 'resource'],
			[`Raw object value`, 'raw']
		]}
		small={true}
		bind:value={option}
	/>
</div>
<div class="mt-1" />
{#if option == 'resource'}
	<ResourcePicker
		bind:value={path}
		resourceType={format.split('-').length > 1 ? format.substring('resource-'.length) : undefined}
	/>
{:else}
	<div class="border rounded p-5">
		<h2 class="mb-5">
			Object of <a target="_blank" href="/resources">resource type</a>
			{resourceTypeName}
		</h2>
		{#if !isString(args)}
			<SchemaForm {schema} bind:isValid bind:args />
		{/if}
	</div>
{/if}
