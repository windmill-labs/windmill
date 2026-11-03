<script module lang="ts">
	import type { Item } from '$lib/utils'
	import type { ResourceType } from '$lib/gen'

	/** The whole value of a resource can be a single `$jsonvar:`/`$var:`/`$res:` string, resolved
	 * at runtime. No form may render over it: a schema form replaces it with empty fields. */
	export function resourceValueReference(value: unknown): string | undefined {
		return typeof value === 'string' &&
			['$jsonvar:', '$var:', '$res:'].some((p) => value.startsWith(p))
			? value
			: undefined
	}

	/** A file-type resource is `{ content }` and a fileset a map of files: neither is a JSON
	 * document a variable could hold whole. */
	export function canReferenceWholeValue(resourceTypeInfo: ResourceType | undefined): boolean {
		return !resourceTypeInfo?.format_extension && !resourceTypeInfo?.is_fileset
	}

	export function resourceValueMenuItems(
		reference: string | undefined,
		actions: { pickVariable: () => void; editFields: () => void }
	): Item[] {
		return reference
			? [
					{ displayName: 'Pick another variable', action: actions.pickVariable },
					{ displayName: 'Edit fields instead', action: actions.editFields }
				]
			: [
					{
						displayName: 'Whole value from a variable',
						description: "The variable's content is parsed as JSON",
						action: actions.pickVariable
					}
				]
	}
</script>

<script lang="ts">
	import { VariableService } from '$lib/gen'
	import { DollarSign, Plus } from 'lucide-svelte'
	import { Button } from './common'
	import ItemPicker from './ItemPicker.svelte'
	import VariableEditor from './VariableEditor.svelte'

	interface Props {
		/** The reference to show, `undefined` when the value is not one: the picker still mounts. */
		reference: string | undefined
		disabled?: boolean
		workspace: string | undefined
		onPick: (reference: string) => void
	}

	let { reference, disabled = false, workspace, onPick }: Props = $props()

	let picker: ItemPicker | undefined = $state(undefined)
	let variableEditor: VariableEditor | undefined = $state(undefined)

	let referencedResource = $derived(reference?.startsWith('$res:') ? reference.slice(5) : undefined)
	let referencedVariable = $derived(
		reference && !referencedResource ? reference.slice(reference.indexOf(':') + 1) : undefined
	)

	export function pickVariable() {
		picker?.openDrawer()
	}
</script>

{#if reference}
	<div class="flex flex-col gap-1 w-full">
		<div class="flex items-center gap-2 h-8 pl-2 pr-0.5 rounded-md bg-surface-secondary">
			<span class="grow truncate text-xs font-mono text-emphasis">{reference}</span>
			{#if !disabled}
				<Button
					iconOnly
					startIcon={{ icon: DollarSign }}
					unifiedSize="sm"
					onClick={pickVariable}
					variant="subtle"
					title="Pick another variable"
				/>
			{/if}
		</div>
		<div class="text-2xs text-hint">
			{#if referencedResource}
				Linked to resource <a
					target="_blank"
					href="/resources{workspace
						? `?workspace=${workspace}`
						: ''}#/resource/{referencedResource}"
					class="text-accent underline">{referencedResource}</a
				>
			{:else}
				Linked to variable <button
					class="text-accent underline font-normal"
					onclick={() => variableEditor?.editVariable?.(referencedVariable ?? '')}
					>{referencedVariable}</button
				>.
				{reference.startsWith('$jsonvar:')
					? 'Its content is parsed as JSON and used as the whole value.'
					: 'Its content is used as the whole value.'}
			{/if}
		</div>
	</div>
{/if}

<ItemPicker
	bind:this={picker}
	pickCallback={(path) => onPick('$jsonvar:' + path)}
	itemName="Variable"
	tooltip="The variable must hold the whole value as JSON. It is parsed when the resource is used."
	documentationLink="https://www.windmill.dev/docs/core_concepts/variables_and_secrets"
	extraField="path"
	loadItems={async () =>
		(await VariableService.listVariable({ workspace: workspace ?? '' })).map((x) => ({
			name: x.path,
			...x
		}))}
>
	{#snippet submission()}
		<div>
			<Button
				variant="default"
				unifiedSize="md"
				startIcon={{ icon: Plus }}
				onClick={() => variableEditor?.initNew?.()}
			>
				New variable
			</Button>
		</div>
	{/snippet}
</ItemPicker>

<VariableEditor bind:this={variableEditor} {workspace} onSaved={() => picker?.reloadItems()} />
