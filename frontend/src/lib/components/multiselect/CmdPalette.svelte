<script lang="ts">
	import { tick } from 'svelte'
	import { fade } from 'svelte/transition'
	import Select from './MultiSelect.svelte'

	export let actions: Action[]
	export let triggers: string[] = [`k`]
	export let close_keys: string[] = [`Escape`]
	export let fade_duration: number = 200 // in ms
	export let style: string = `` // for dialog
	// for span in option slot, has no effect when passing a slot
	export let span_style: string = ``
	export let open: boolean = false
	export let dialog: HTMLDialogElement | null = null
	export let input: HTMLInputElement | null = null
	export let placeholder: string = `Filter actions...`

	type Action = { label: string; action: () => void }

	async function toggle(event: KeyboardEvent) {
		if (triggers.includes(event.key) && event.metaKey && !open) {
			// open on cmd+trigger
			open = true
			await tick() // wait for dialog to open and input to be mounted
			input?.focus()
		} else if (close_keys.includes(event.key) && open) {
			open = false
		}
	}

	function close_if_outside(event: MouseEvent) {
		if (open && !dialog?.contains(event.target as Node)) {
			open = false
		}
	}

	function trigger_action_and_close(event: CustomEvent<{ option: Action }>) {
		event.detail.option.action(event.detail.option.label)
		open = false
	}
</script>

<svelte:window on:keydown={toggle} on:click={close_if_outside} />

{#if open}
	<dialog open bind:this={dialog} transition:fade={{ duration: fade_duration }} {style}>
		<Select
			options={actions}
			bind:input
			{placeholder}
			on:add={trigger_action_and_close}
			on:keydown={toggle}
			{...$$restProps}
			let:option
		>
			<!-- wait for https://github.com/sveltejs/svelte/pull/8304 -->
			<slot>
				<span style={span_style}>{option.label}</span>
			</slot>
		</Select>
	</dialog>
{/if}

<style>
	:where(dialog) {
		position: fixed;
		top: 30%;
		border: none;
		padding: 0;
		background-color: transparent;
		display: flex;
		color: white;
		z-index: 10;
		font-size: 2.4ex;
	}
	dialog :global(div.multiselect) {
		--sms-bg: var(--sms-options-bg);
		--sms-width: min(20em, 90vw);
		--sms-max-width: none;
		--sms-placeholder-color: lightgray;
		--sms-options-margin: 1px 0;
		--sms-options-border-radius: 0 0 1ex 1ex;
	}
</style>
