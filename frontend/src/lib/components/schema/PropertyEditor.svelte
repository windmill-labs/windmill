<script lang="ts">
	import autosize from '$lib/autosize'
	import { shouldDisplayPlaceholder } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import ArrayTypeNarrowing from '../ArrayTypeNarrowing.svelte'
	import Label from '../Label.svelte'
	import NumberTypeNarrowing from '../NumberTypeNarrowing.svelte'
	import StringTypeNarrowing from '../StringTypeNarrowing.svelte'
	import Tooltip from '../Tooltip.svelte'

	import EditableSchemaForm from '../EditableSchemaForm.svelte'
	import { deepEqual } from 'fast-equals'
	import type { EnumType } from '$lib/common'

	export let description: string = ''
	export let format: string = ''
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let pattern: undefined | string = undefined
	export let enum_: EnumType = undefined
	export let extra: Record<string, any> = {}
	export let minW = true
	export let customErrorMessage: string | undefined = undefined
	export let title: string | undefined = undefined
	export let placeholder: string | undefined = undefined
	export let properties: Record<string, any> = {}
	export let isFlowInput: boolean = false
	export let isAppInput: boolean = false
	export let order: string[] = []
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object'
				contentEncoding?: 'base64'
				enum?: string[]
				multiselect?: string[]
		  }
		| undefined = undefined

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

	let schema = {
		properties,
		order
	}

	$: (properties || order) && updateSchema()

	function updateSchema() {
		if (!deepEqual(schema.properties, properties) || !deepEqual(schema.order, order)) {
			schema = {
				properties,
				order
			}
		}
	}
</script>

<div class="flex flex-row items-center justify-between w-full gap-2">
	<!-- svelte-ignore a11y-autofocus -->
	<div class={twMerge('flex flex-col w-full', 'gap-4', minW ? 'min-w-[250px]' : '')}>
		<slot name="typeeditor" />

		<Label label="Description">
			<textarea
				use:autosize
				rows="2"
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
			<div>
				<Label label="Field settings">
					<div>
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
						{:else if type == 'object' && !format?.startsWith('resource-') && !isFlowInput && !isAppInput}
							<div class="border">
								<EditableSchemaForm on:change noPreview bind:schema uiOnly jsonEnabled={false} />
							</div>
						{:else}
							<div class="text-tertiary text-xs">No settings available for this field type</div>
						{/if}
					</div>
				</Label>
			</div>
		{/if}

		<slot />
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
