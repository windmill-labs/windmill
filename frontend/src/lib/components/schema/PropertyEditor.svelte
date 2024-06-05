<script lang="ts">
	import autosize from '$lib/autosize'
	import { shouldDisplayPlaceholder } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import ArrayTypeNarrowing from '../ArrayTypeNarrowing.svelte'
	import Label from '../Label.svelte'
	import NumberTypeNarrowing from '../NumberTypeNarrowing.svelte'
	import ObjectTypeNarrowing from '../ObjectTypeNarrowing.svelte'
	import Section from '../Section.svelte'
	import StringTypeNarrowing from '../StringTypeNarrowing.svelte'
	import Tooltip from '../Tooltip.svelte'

	export let description: string = ''
	export let format: string = ''
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let pattern: undefined | string = undefined
	export let enum_: string[] | undefined = undefined
	export let editableSchema: { i: number; total: number } | undefined = undefined
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object'
				contentEncoding?: 'base64'
				enum?: string[]
				multiselect?: string[]
		  }
		| undefined = undefined
	export let extra: Record<string, any> = {}
	export let minW = true
	export let customErrorMessage: string | undefined = undefined
	export let title: string | undefined = undefined
	export let placeholder: string | undefined = undefined

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

<div class="flex flex-row items-center justify-between w-full gap-2">
	<!-- svelte-ignore a11y-autofocus -->
	<div class={twMerge('flex flex-col w-full', 'gap-1', minW ? 'min-w-[250px]' : '')}>
		{#if editableSchema}
			<Label label="Description">
				<textarea
					use:autosize
					rows="1"
					bind:value={description}
					on:keydown={onKeyDown}
					placeholder="Field description"
				/>
			</Label>

			<Label label="Custom Title" class="w-full">
				<svelte:fragment slot="header">
					<Tooltip light>Will be displayed in the UI instead of the field name.</Tooltip>
				</svelte:fragment>
				<input bind:value={title} on:keydown={onKeyDown} placeholder="Field title" />
			</Label>

			<Label label="Placeholder">
				<svelte:fragment slot="header">
					<Tooltip light>
						Will be displayed in the input field when the field is empty. If not set, the default
						value will be used. The placeholder is disabled depending on the field typ, format, etc.
					</Tooltip>
				</svelte:fragment>

				<textarea
					placeholder="Enter a placeholder"
					rows="1"
					bind:value={placeholder}
					disabled={!shouldDisplayPlaceholder(type, format, enum_, contentEncoding, pattern, extra)}
				/>
			</Label>

			{#if type == 'array'}
				<ArrayTypeNarrowing bind:itemsType />
			{:else if type == 'string' || ['number', 'integer', 'object'].includes(type ?? '')}
				<Section label="Field settings" small>
					<div class="mt-2">
						{#if type == 'string'}
							<StringTypeNarrowing
								bind:customErrorMessage
								bind:format
								bind:pattern
								bind:enum_
								bind:contentEncoding
								bind:password={extra['password']}
								bind:minRows={extra['minRows']}
								bind:disableCreate={extra['disableCreate']}
								bind:disableVariablePicker={extra['disableVariablePicker']}
								bind:dateFormat={extra['dateFormat']}
							/>
						{:else if type == 'number' || type == 'integer'}
							<NumberTypeNarrowing
								bind:min={extra['min']}
								bind:max={extra['max']}
								bind:currency={extra['currency']}
								bind:currencyLocale={extra['currencyLocale']}
							/>
						{:else if type == 'object'}
							<ObjectTypeNarrowing bind:format />
						{/if}
					</div>
				</Section>
			{/if}

			<slot />
		{/if}
	</div>
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
