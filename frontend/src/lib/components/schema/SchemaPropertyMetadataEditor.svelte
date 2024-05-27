<script lang="ts">
	import { shouldDisplayPlaceholder } from '$lib/utils'
	import ArrayTypeNarrowing from '../ArrayTypeNarrowing.svelte'
	import Label from '../Label.svelte'
	import NumberTypeNarrowing from '../NumberTypeNarrowing.svelte'
	import ObjectTypeNarrowing from '../ObjectTypeNarrowing.svelte'
	import StringTypeNarrowing from '../StringTypeNarrowing.svelte'
	import Section from '$lib/components/Section.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	export let format: string = ''
	export let contentEncoding: 'base64' | 'binary' | undefined = undefined
	export let type: string | undefined = undefined
	export let required = false
	export let pattern: undefined | string = undefined
	export let enum_: string[] | undefined = undefined
	export let itemsType:
		| {
				type?: 'string' | 'number' | 'bytes' | 'object'
				contentEncoding?: 'base64'
				enum?: string[]
				multiselect?: string[]
		  }
		| undefined = undefined
	export let extra: Record<string, any> = {}
	export let customErrorMessage: string | undefined = undefined
	export let placeholder: string | undefined = undefined
</script>

{#if type == 'array'}
	<ArrayTypeNarrowing bind:itemsType />
{:else if type == 'string' || ['number', 'integer', 'object'].includes(type ?? '')}
	<div class="mt-8">
		<Section label="Arguments" small>
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
			{#if shouldDisplayPlaceholder(type, format, enum_, contentEncoding, pattern)}
				<Label label="Placeholder" class="pt-2">
					<textarea placeholder="Enter a placeholder" rows="1" bind:value={placeholder} />
				</Label>
			{/if}
			{#if !required && type === 'string'}
				<div class="mt-2 border-t pt-4">
					<Toggle
						options={{
							right: 'Nullable',
							rightTooltip: 'If enabled, the default value will be null and not an empty string.'
						}}
						size="xs"
						bind:checked={extra.nullable}
					/>
				</div>
			{/if}
		</Section>
	</div>
{/if}
