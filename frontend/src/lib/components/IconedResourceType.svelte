<script>
	import { FileText, FolderOpen } from 'lucide-svelte'
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
	 * @property {boolean} [isFileset]
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
		formatExtension = undefined,
		isFileset = false
	} = $props()

	let iconComponent = $derived(
		name
			? name === 'teams'
				? APP_TO_ICON_COMPONENT.ms_teams_webhook
				: APP_TO_ICON_COMPONENT[name] || APP_TO_ICON_COMPONENT[name.split('_')[0]]
			: undefined
	)

	let widthInPixels = $derived(parseInt(width))
</script>

<div class="truncate flex flex-row gap-2 items-center {center ? 'justify-center ' : ''} -pl-2">
	{#if !silent && !after}
		{name}
	{/if}
	{#if iconComponent}
		{@const SvelteComponent = iconComponent}
		<span class={isSelected ? 'text-secondary' : 'text-secondary'}>
			<SvelteComponent {height} {width} size={widthInPixels} />
		</span>
	{:else if isFileset}
		<span class={isSelected ? 'text-secondary' : 'text-secondary grayscale'}>
			<FolderOpen {height} {width} />
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
