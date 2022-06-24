<script lang="ts">
	import Required from './Required.svelte'

	export let label: string
	export let format: string = ''
	export let contentEncoding = ''
	export let type: string | undefined = undefined
	export let required = false
	export let itemsType:
		| { type?: 'string' | 'number' | 'bytes'; contentEncoding?: 'base64' }
		| undefined = undefined
</script>

<h3>
	<span class="font-semibold">
		{label}<Required {required} />
	</span>
	<span class="text-sm italic text-indigo-800"
		>({type ?? 'any'}{contentEncoding && contentEncoding != ''
			? `, encoding: ${contentEncoding}`
			: ''}{format && format != '' ? `, format: ${format}` : ''}{itemsType?.type
			? ` of ${itemsType?.type}s`
			: ''})</span
	>
</h3>
