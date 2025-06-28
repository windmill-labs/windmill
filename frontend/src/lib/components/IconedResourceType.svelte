<script>
	import { FileText } from 'lucide-svelte'
	import { APP_TO_ICON_COMPONENT } from './icons'
	/**
	 * @typedef {Object} Props
	 * @property {any} name
	 * @property {boolean} [silent]
	 * @property {boolean} [after]
	 * @property {string} [height]
	 * @property {string} [width]
	 * @property {boolean} [center]
	 * @property {boolean} [isSelected]
	 * @property {any} [formatExtension]
	 */

	/** @type {Props} */
	let {
		name,
		silent = false,
		after = false,
		height = '24px',
		width = '24px',
		center = false,
		isSelected = false,
		formatExtension = undefined
	} = $props()

	let iconComponent = $derived(
		name === 'teams'
			? APP_TO_ICON_COMPONENT.ms_teams_webhook
			: APP_TO_ICON_COMPONENT[name] || APP_TO_ICON_COMPONENT[name.split('_')[0]]
	)
</script>

<div class="truncate flex flex-row gap-2 {center ? 'justify-center items-center' : ''}  -pl-2">
	{#if !silent && !after}
		{name}
	{/if}
	{#if iconComponent}
		{@const SvelteComponent = iconComponent}
		<span class={isSelected ? 'text-secondary' : 'text-secondary grayscale'}>
			<SvelteComponent {height} {width} />
		</span>
	{:else if formatExtension}
		<span class={isSelected ? 'text-secondary' : 'text-secondary grayscale'}>
			<FileText {height} {width} />
		</span>
	{:else}
		<span style="width: {width}; height: {height}" class="bg-gray-100 rounded-full"></span>
	{/if}
	{#if !silent && after}
		{name}
	{/if}
</div>
