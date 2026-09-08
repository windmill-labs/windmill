<script lang="ts">
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import type { Label } from '$lib/gen'
	import LabelForm from './LabelForm.svelte'

	interface Props {
		/** The workspace the label belongs to — not necessarily the one being navigated. */
		workspace: string | undefined
		/** The name that was saved. */
		onSave?: (name: string) => void
	}

	let { workspace, onSave }: Props = $props()

	let isOpen = $state(false)
	let existing: Label | undefined = $state()
	// Bumped on every open so the form remounts with fresh fields rather than
	// keeping what the previous label left in them.
	let opened = $state(0)

	export function initNew() {
		existing = undefined
		opened += 1
		isOpen = true
	}

	export function initEdit(label: Label) {
		existing = label
		opened += 1
		isOpen = true
	}
</script>

<Modal2
	bind:isOpen
	formStyling
	fixedWidth="sm"
	fixedHeight="adaptive"
	title={existing ? 'Edit label' : 'New label'}
>
	{#key opened}
		<LabelForm
			{workspace}
			{existing}
			autofocus={existing == undefined}
			onSaved={(name) => {
				isOpen = false
				onSave?.(name)
			}}
		/>
	{/key}
</Modal2>
