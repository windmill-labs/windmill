<script lang="ts">
	import { Plus, X } from 'lucide-svelte'
	import { Button } from './common'
	import { fade } from 'svelte/transition'
	import Label from './Label.svelte'
	import ResourceTypePicker from './ResourceTypePicker.svelte'
	import Badge from './common/badge/Badge.svelte'
	import Alert from './common/alert/Alert.svelte'
	import EditableSchemaDrawer from './schema/EditableSchemaDrawer.svelte'
	import type { SchemaProperty } from '$lib/common'

	export let canEditResourceType: boolean = false
	export let originalType: string | undefined = undefined
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object' | 'resource'
				contentEncoding?: 'base64'
				enum?: string[]
				resourceType?: string
				properties?: { [name: string]: SchemaProperty }
		  }
		| undefined

	let selected:
		| 'string'
		| 'number'
		| 'object'
		| 'bytes'
		| 'enum'
		| 'resource'
		| 's3object'
		| undefined =
		itemsType?.type != 'string'
			? itemsType?.type == 'object' && itemsType?.resourceType == 's3object'
				? 's3object'
				: itemsType?.type
			: Array.isArray(itemsType?.enum)
				? 'enum'
				: itemsType?.contentEncoding == 'base64'
					? 'bytes'
					: 'string'

	let schema = {
		properties: itemsType?.properties || {},
		order: Object.keys(itemsType?.properties || {}),
		required: Object.values(itemsType?.properties || {}).map((p) => p.required)
	}

	function updateItemsType() {
		itemsType = {
			...itemsType,
			properties: schema.properties,
			type: 'object'
		}
	}
</script>

{#if canEditResourceType || originalType == 'string[]' || originalType == 'object[]'}
	<Label label="Items type">
		<select
			bind:value={selected}
			on:change={() => {
				if (selected == 'enum') {
					itemsType = { type: 'string', enum: [] }
				} else if (selected == 'string') {
					itemsType = { type: 'string' }
				} else if (selected == 'number') {
					itemsType = { type: 'number' }
				} else if (selected == 'object') {
					itemsType = { ...itemsType, type: 'object' }
				} else if (selected == 'bytes') {
					itemsType = { type: 'string', contentEncoding: 'base64' }
				} else if (selected == 'resource') {
					itemsType = { type: 'resource', resourceType: itemsType?.resourceType }
				} else if (selected == 's3object') {
					itemsType = { type: 'object', resourceType: 's3object' }
				} else {
					itemsType = undefined
				}
			}}
			id="array-type-narrowing"
		>
			<option value="string"> Items are strings</option>
			<option value="enum">Items are strings from an enum</option>
			{#if originalType != 'string[]'}
				<option value="s3object">Items are S3 objects</option>
				<option value="object"> Items are objects (JSON)</option>
				<option value="resource"> Items are resources</option>
				<option value="number">Items are numbers</option>
				<option value="bytes">Items are bytes</option>
			{/if}
		</select>
	</Label>
{:else if itemsType?.resourceType}
	<Label label="Resource type">
		<Badge color="blue">
			{itemsType?.resourceType}
		</Badge>
	</Label>
{/if}

{#if itemsType?.type === 'resource'}
	<Alert
		type="warning"
		title="Value"
		size="xs"
		tooltip="Learn how to use the SDK to get the resource by using the path."
		documentationLink="https://www.windmill.dev/docs/code_editor/add_variables_resources#fetching-them-from-within-a-script-by-using-the-wmill-client-in-the-respective-language"
	>
		The value passed is the path of the resource, not the resource itself. You can use the SDK to
		get the resource by using the path.
	</Alert>
{/if}

{#if canEditResourceType && itemsType?.type == 'resource'}
	<ResourceTypePicker bind:value={itemsType.resourceType} />
{/if}

{#if Array.isArray(itemsType?.enum)}
	<label for="input" class="text-secondary text-xs">
		Enums
		<div class="flex flex-col gap-1">
			{#each itemsType?.enum || [] as _, index}
				<div class="flex flex-row max-w-md gap-1 items-center">
					<input id="input" type="text" bind:value={itemsType.enum[index]} />
					<div>
						<button
							transition:fade|local={{ duration: 100 }}
							class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
							on:click={() => {
								if (itemsType && itemsType.enum) {
									const enumValue = itemsType.enum[index]
									itemsType.enum = itemsType.enum.filter((el) => el !== enumValue)
								}
							}}
						>
							<X size={14} />
						</button>
					</div>
				</div>
			{/each}
		</div>
		{#if canEditResourceType || originalType == 'string[]' || originalType == 'object[]'}
			<div class="flex flex-row mb-1 mt-2">
				<Button
					color="light"
					variant="border"
					size="sm"
					on:click={() => {
						if (itemsType?.enum) {
							let enum_ = itemsType.enum
							let choice = `choice ${enum_?.length ? enum_?.length + 1 : 1}`
							itemsType.enum = itemsType.enum ? itemsType.enum.concat(choice) : [choice]
						}
					}}
				>
					<Plus size={14} />
				</Button>
				<Button
					color="light"
					variant="border"
					size="sm"
					btnClasses="ml-2"
					on:click={() => itemsType?.enum && (itemsType.enum = undefined)}
				>
					Clear
				</Button>
			</div>
		{/if}
	</label>
{/if}

{#if selected === 'object'}
	<EditableSchemaDrawer
		bind:schema
		on:change={() => {
			updateItemsType()
		}}
	/>
{/if}
