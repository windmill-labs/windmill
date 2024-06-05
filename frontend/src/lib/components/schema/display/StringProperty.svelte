<script lang="ts">
	import { DollarSign } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import type ItemPicker from '../../ItemPicker.svelte'
	import type VariableEditor from '../../VariableEditor.svelte'
	import { twMerge } from 'tailwind-merge'

	import Password from '$lib/components/Password.svelte'
	import PasswordArgInput from '$lib/components/PasswordArgInput.svelte'
	import autosize from '$lib/autosize'

	export let label: string = ''
	export let value: any
	export let valid = true
	export let disabled = false
	export let autofocus: boolean | null = null
	export let password = false
	export let pickForField: string | undefined = undefined
	export let variableEditor: VariableEditor | undefined = undefined
	export let itemPicker: ItemPicker | undefined = undefined
	export let extra: Record<string, any> = {}
	export let onlyMaskPassword = false
	export let placeholder: string | undefined = undefined
	export let defaultValue: any = undefined

	const dispatch = createEventDispatcher()

	let el: HTMLTextAreaElement | undefined = undefined

	export function focus() {
		el?.focus()
		if (el) {
			el.style.height = '5px'
			el.style.height = el.scrollHeight + 50 + 'px'
		}
	}

	function onKeyDown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key == 'Enter') {
			return
		}
		e.stopPropagation()
	}
</script>

<div class="flex flex-col w-full">
	<div class="flex flex-row w-full items-center justify-between relative">
		{#if password || extra?.['password'] == true}
			{#if onlyMaskPassword}
				<Password
					{disabled}
					bind:password={value}
					placeholder={placeholder ?? defaultValue ?? ''}
				/>
			{:else}
				<PasswordArgInput {disabled} bind:value />
			{/if}
		{:else}
			{#key extra?.['minRows']}
				<!-- svelte-ignore a11y-autofocus -->
				<textarea
					{autofocus}
					rows={extra?.['minRows'] ? extra['minRows']?.toString() : '1'}
					bind:this={el}
					on:focus={(e) => {
						dispatch('focus')
					}}
					on:blur={(e) => {
						dispatch('blur')
					}}
					use:autosize
					on:keydown={onKeyDown}
					{disabled}
					class={twMerge(
						'w-full',
						valid
							? ''
							: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-3'
					)}
					placeholder={placeholder ?? defaultValue ?? ''}
					bind:value
				/>
			{/key}
			{#if !disabled && itemPicker && extra?.['disableVariablePicker'] != true}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<button
					class="absolute right-1 top-1 py-1 min-w-min !px-2 items-center text-gray-800 bg-surface-secondary border rounded center-center hover:bg-gray-300 transition-all cursor-pointer"
					on:click={() => {
						pickForField = label
						itemPicker?.openDrawer?.()
					}}
					title="Insert a Variable"
				>
					<DollarSign class="!text-tertiary" size={14} />
				</button>
			{/if}
		{/if}
	</div>
	{#if variableEditor}
		<div class="text-sm text-tertiary">
			{#if value && typeof value == 'string' && value?.startsWith('$var:')}
				Linked to variable <button
					class="text-blue-500 underline"
					on:click={() => variableEditor?.editVariable?.(value.slice(5))}
				>
					{value.slice(5)}</button
				>
			{/if}
		</div>
	{/if}
</div>

<style>
	input::-webkit-outer-spin-button,
	input::-webkit-inner-spin-button {
		-webkit-appearance: none !important;
		margin: 0;
	}

	/* Firefox */
	input[type='number'] {
		-moz-appearance: textfield !important;
	}
</style>
