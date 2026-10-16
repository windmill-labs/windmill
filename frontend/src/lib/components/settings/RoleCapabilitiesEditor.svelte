<script lang="ts">
	import { Badge, Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type {
		AuthorLevel,
		RoleCapabilities,
		RoleCapabilitiesPatch,
		RoleVisibility
	} from '$lib/gen'
	import {
		applyCapabilitiesPatch,
		capabilitiesError,
		isEmptyPatch,
		VISIBILITY_KEYS
	} from '$lib/workspaceRoles'
	import { RotateCcw } from 'lucide-svelte'

	interface Props {
		/** The layer an unset field falls through to. */
		base: RoleCapabilities
		/** Fields whose `base` value is not known here, as `code` or `visibility.runs`. */
		unknownBase?: string[]
		patch: RoleCapabilitiesPatch
		/** What the layer below is called: "default" on the instance, "instance" in a workspace. */
		baseName: string
		/** Marks a field this layer sets, and one it leaves to `base`. */
		setText: string
		inheritedText: string
		disabled?: boolean
		onchange?: (patch: RoleCapabilitiesPatch) => void
	}

	let {
		base,
		unknownBase = [],
		patch,
		baseName,
		setText,
		inheritedText,
		disabled = false,
		onchange
	}: Props = $props()

	const effective = $derived(applyCapabilitiesPatch(base, patch))
	const error = $derived(capabilitiesError(effective))

	type BoolKey = 'code' | 'manage_schedules' | 'manage_triggers'
	type LevelKey = 'flows' | 'apps'

	function boolValue(path: string): boolean {
		if (path.startsWith('visibility.')) {
			return effective.visibility[path.slice('visibility.'.length) as keyof RoleVisibility]
		}
		return effective[path as BoolKey]
	}

	function isSet(path: string): boolean {
		if (path.startsWith('visibility.')) {
			return patch.visibility?.[path.slice('visibility.'.length)] != null
		}
		return patch[path as keyof RoleCapabilitiesPatch] != null
	}

	function update(path: string, value: boolean | AuthorLevel | undefined) {
		const next: RoleCapabilitiesPatch = structuredClone($state.snapshot(patch))
		if (path.startsWith('visibility.')) {
			const vis = { ...(next.visibility ?? {}) }
			const k = path.slice('visibility.'.length)
			if (value === undefined) delete vis[k]
			else vis[k] = value as boolean
			next.visibility = Object.keys(vis).length > 0 ? vis : undefined
			if (next.visibility === undefined) delete next.visibility
		} else if (value === undefined) {
			delete next[path as keyof RoleCapabilitiesPatch]
		} else {
			;(next as Record<string, unknown>)[path] = value
		}
		onchange?.(next)
	}

	function resetAll() {
		onchange?.({})
	}

	const authoring: { key: BoolKey | LevelKey; label: string; description: string }[] = [
		{
			key: 'code',
			label: 'Write code',
			description:
				'Author scripts and run code of their own: previews, dependency jobs, dynamic selects. Required for full flow or app authoring.'
		},
		{
			key: 'flows',
			label: 'Flows',
			description: 'Compose assembles deployed scripts and flows without code of their own.'
		},
		{
			key: 'apps',
			label: 'Apps',
			description:
				'Compose builds full-code apps out of deployed runnables, always sandboxed. Low-code apps need full.'
		}
	]

	const management: { key: BoolKey; label: string; description: string }[] = [
		{
			key: 'manage_schedules',
			label: 'Manage schedules',
			description: 'Create, edit and delete schedules where folder permissions allow.'
		},
		{
			key: 'manage_triggers',
			label: 'Manage triggers',
			description: 'Create, edit and delete triggers where folder permissions allow.'
		}
	]
</script>

{#snippet status(path: string)}
	<div class="flex items-center justify-end gap-1 w-32 shrink-0">
		{#if isSet(path)}
			<Badge color="blue" verySmall>{setText}</Badge>
			<Button
				variant="subtle"
				unifiedSize="xs"
				iconOnly
				startIcon={{ icon: RotateCcw }}
				title="Reset to the {baseName} value"
				aria-label="Reset to the {baseName} value"
				{disabled}
				onclick={() => update(path, undefined)}
			/>
		{:else if unknownBase.includes(path)}
			<span class="text-2xs text-hint text-right">{inheritedText}, shown after saving</span>
		{:else}
			<span class="text-2xs text-hint">{inheritedText}</span>
		{/if}
	</div>
{/snippet}

{#snippet row(label: string, description: string | undefined, path: string)}
	<div class="flex items-center gap-4 py-1.5">
		<div class="flex flex-col grow min-w-0">
			<span class="text-xs font-semibold text-emphasis">{label}</span>
			{#if description}
				<span class="text-2xs text-secondary">{description}</span>
			{/if}
		</div>
		<div class="flex justify-end w-60 shrink-0">
			{#if path === 'flows' || path === 'apps'}
				{@const key = path as LevelKey}
				<ToggleButtonGroup
					{disabled}
					selected={effective[key]}
					onSelected={(v) => update(key, v as AuthorLevel)}
				>
					{#snippet children({ item })}
						<ToggleButton value="none" label="None" small {item} />
						<ToggleButton value="compose" label="Compose" small {item} />
						<ToggleButton
							value="full"
							label="Full"
							small
							disabled={!effective.code}
							tooltip={effective.code ? undefined : 'Requires the code capability'}
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>
			{:else}
				<Toggle
					size="xs"
					{disabled}
					checked={boolValue(path)}
					on:change={(e) => update(path, e.detail)}
				/>
			{/if}
		</div>
		{@render status(path)}
	</div>
{/snippet}

<div class="flex flex-col gap-4">
	<div class="flex flex-col divide-y">
		<span class="text-xs font-semibold text-secondary pb-1">Authoring</span>
		{#each authoring as f (f.key)}
			{@render row(f.label, f.description, f.key)}
		{/each}
	</div>
	{#if error}
		<span class="text-xs text-red-600 dark:text-red-400">{error}</span>
	{/if}

	<div class="flex flex-col divide-y">
		<span class="text-xs font-semibold text-secondary pb-1">Management</span>
		{#each management as f (f.key)}
			{@render row(f.label, f.description, f.key)}
		{/each}
	</div>

	<div class="flex flex-col divide-y">
		<span class="text-xs font-semibold text-secondary pb-1">
			Pages in their menu
			<span class="font-normal">(hides a page, does not block the API)</span>
		</span>
		{#each VISIBILITY_KEYS as v (v.key)}
			{@render row(v.label, undefined, `visibility.${v.key}`)}
		{/each}
	</div>

	{#if !isEmptyPatch(patch)}
		<div>
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: RotateCcw }}
				{disabled}
				onclick={resetAll}
			>
				Reset all to the {baseName} values
			</Button>
		</div>
	{/if}
</div>
