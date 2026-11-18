<script lang="ts">
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import { GripVertical, Plus } from 'lucide-svelte'
	import { randomUUID } from '$lib/utils/uuid'
	import type { S3ResourceSettingsItem } from '$lib/workspace_settings'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import ClearableInput from '../common/clearableInput/ClearableInput.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import MultiSelect from '../select/MultiSelect.svelte'

	type Rule = NonNullable<S3ResourceSettingsItem['advancedPermissions']>[number]

	let { rules = $bindable() }: { rules: Rule[] | undefined } = $props()

	// svelte-dnd-action keys its items by `id`. Wrapping the rules rather than adding
	// an `id` to them keeps the key out of what gets persisted to the backend.
	let items = $state((rules ?? []).map((rule) => ({ id: randomUUID(), rule })))

	$effect(() => {
		rules = items.map((item) => item.rule)
	})

	// Evaluation stops at the first rule whose pattern matches, so a rule matching every
	// path makes everything below it dead — most often the `**/*` deny-all the default
	// ruleset ends with.
	const CATCH_ALL_PATTERNS = ['**/*', '**', '*']
	let catchAllIdx = $derived.by(() => {
		const idx = items.findIndex((item) => CATCH_ALL_PATTERNS.includes(item.rule.pattern.trim()))
		return idx === -1 || idx === items.length - 1 ? undefined : idx
	})
	let shadowWarning = $derived.by(() => {
		if (catchAllIdx === undefined) return undefined
		const shadowed =
			catchAllIdx === items.length - 2
				? `Rule ${items.length} is`
				: `Rules ${catchAllIdx + 2} to ${items.length} are`
		return `${shadowed} never evaluated: rule ${catchAllIdx + 1} (${items[catchAllIdx].rule.pattern.trim()}) already matches every path`
	})

	const flipDurationMs = 200
</script>

<Alert title="Rules are evaluated in order">
	The first rule whose pattern matches the path decides what is allowed — drag rules to reorder
	them. A path matched by no rule is denied.
	<br /><br />
	Standard Unix-style glob syntax is supported. The following will be interpolated:
	<ul class="list-disc pl-6">
		<li><code>{'{username}'}</code> : Nickname of the user doing the request</li>
		<li><code>{'{group}'}</code> : Any group that the user belongs to</li>
		<li><code>{'{folder_read}'}</code> : Any folder that the user has read access to</li>
		<li><code>{'{folder_write}'}</code> : Any folder that the user has write access to</li>
	</ul>
	<br />
	Note that changes may take up to 1 minute to propagate due to cache invalidation
</Alert>

<div class="flex-1 overflow-y-auto">
	<section
		class="flex flex-col gap-3"
		use:dragHandleZone={{ items, flipDurationMs, dropTargetStyle: {} }}
		onconsider={(e) => (items = e.detail.items)}
		onfinalize={(e) => (items = e.detail.items)}
	>
		{#each items as item, idx (item.id)}
			{@const shadowed = catchAllIdx !== undefined && idx > catchAllIdx}
			<!-- The transparent border is carried by every row so flagging one doesn't shift the columns. -->
			<div
				class="flex gap-2 items-center pl-2 border-l-2 {shadowed
					? 'border-red-500'
					: 'border-transparent'}"
			>
				<div
					class="shrink-0 flex items-center gap-1 cursor-move {shadowed
						? 'text-red-500'
						: 'text-secondary'}"
					use:dragHandle
					aria-label="Reorder rule {idx + 1}"
				>
					<GripVertical size={16} />
					<span class="text-2xs whitespace-nowrap tabular-nums">Rule {idx + 1}</span>
				</div>
				<ClearableInput bind:value={item.rule.pattern} placeholder="Pattern" />
				<MultiSelect
					items={[{ value: 'read' }, { value: 'write' }, { value: 'delete' }, { value: 'list' }]}
					bind:value={item.rule.allow}
					class="w-[20rem]"
					placeholder="Deny all access"
					hideMainClearBtn
				/>
				<CloseButton onClick={() => (items = items.filter((_, i) => i !== idx))} />
			</div>
		{/each}
	</section>
</div>
{#if shadowWarning}
	<Alert type="error" size="xs" title={shadowWarning} />
{/if}
<Button
	unifiedSize="sm"
	variant="default"
	startIcon={{ icon: Plus }}
	on:click={() => (items = [...items, { id: randomUUID(), rule: { pattern: '', allow: [] } }])}
>
	Add permission rule
</Button>
