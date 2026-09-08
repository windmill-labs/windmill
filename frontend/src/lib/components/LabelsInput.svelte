<script lang="ts">
	import Button from './common/button/Button.svelte'
	import { Plus, Tag } from 'lucide-svelte'
	import type { Label } from '$lib/gen'
	import LabelBadge from './labels/LabelBadge.svelte'
	import LabelPickerDropdown from './labels/LabelPickerDropdown.svelte'
	import { loadLabels } from './labels/labelStore'

	interface Props {
		labels: string[] | undefined
		/**
		 * The workspace the edited item lives in. Not always the navigated one — a
		 * session editor operates on a fork while the sidebar still browses the
		 * origin — so a color picked here must be written where the item is.
		 */
		workspace: string | undefined
		onchange?: () => void
		class?: string
	}

	let { labels = $bindable(), workspace, onchange, class: clazz = '' }: Props = $props()

	let adding = $state(false)
	let existingLabels: Label[] = $state([])
	let picker: LabelPickerDropdown | undefined = $state()

	// The chips need colors before the picker is ever opened, and `loadLabels` is
	// cached per workspace, so this costs one request per workspace.
	$effect(() => {
		if (workspace) {
			loadLabels(workspace).catch(() => {})
		}
	})

	// Always refetches: labels appear the moment anyone types a new one onto an
	// item, so a cached vocabulary would stop suggesting labels created since the
	// page loaded.
	async function loadExistingLabels() {
		if (!workspace) return
		try {
			existingLabels = await loadLabels(workspace, true)
		} catch {}
	}

	function startAdding() {
		picker?.reset()
		adding = true
		loadExistingLabels()
	}

	function addLabel(value: string) {
		const v = value.trim().slice(0, 50)
		if (!v) return
		if (!labels) {
			labels = []
		}
		if (!labels.includes(v)) {
			labels = [...labels, v]
			onchange?.()
		}
	}

	function removeLabel(label: string) {
		if (labels) {
			labels = labels.filter((l) => l !== label)
			onchange?.()
		}
	}
</script>

<div class="inline-flex items-center gap-1 ml-0.5 h-5 {clazz}">
	{#each labels ?? [] as label (label)}
		<LabelBadge {label} {workspace} onRemove={removeLabel} />
	{/each}
	{#if adding}
		<div class="w-40">
			<LabelPickerDropdown
				bind:this={picker}
				bind:open={adding}
				{workspace}
				labels={existingLabels}
				taken={labels ?? []}
				onPick={addLabel}
				onLabelsChanged={loadExistingLabels}
				inputClass="!h-5 !min-h-0 !py-0 text-2xs"
			/>
		</div>
	{:else}
		<Button
			variant="subtle"
			unifiedSize="xs"
			startIcon={{ icon: Tag }}
			endIcon={{ icon: Plus, props: { size: 8 } }}
			btnClasses="!gap-0.5"
			aria-label="Add label"
			onClick={startAdding}
		/>
	{/if}
</div>
