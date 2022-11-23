<script lang="ts">
	import {
		faArchive,
		faEdit,
		faList,
		faPlay,
		faShare,
		faTrash
	} from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	export let category: 'delete' | 'list' | 'run' | 'add' | 'edit' | 'archive' | 'share'
	export let disabled: boolean = false

	const colors = {
		red: 'bg-transparent hover:bg-red-600 text-red-600 font-normal hover:text-white border-red-600 hover:border-transparent rounded',
		blue: 'hover:bg-blue-500 hover:text-white'
	}

	const getCategoryClasses = () => {
		if (category === 'delete' || category === 'archive') return colors.red
		else if (category === 'edit' || category === 'list' || category === 'share') return colors.blue
		return ''
	}
</script>

<button
	class="{$$props.class} inline-flex items-center bg-[#5e81ac] hover:bg-blue-700 text-white 
	font-bold py-1 px-2 border rounded border-blue-500 hover:border-blue-700 w-min min-w-max 
	text-sm {getCategoryClasses()}"
	on:click
	{disabled}
>
	<div class="inline-flex items-center justify-center">
		{#if category === 'delete'}
			<Icon data={faTrash} scale={0.5} />
			<slot name="text">
				<span class="pl-1">Delete</span>
			</slot>
		{:else if category === 'list'}
			<Icon data={faList} scale={0.5} />
			<slot name="text">
				<span class="pl-1">List</span>
			</slot>
		{:else if category === 'run'}
			<Icon data={faPlay} scale={0.5} />
			<slot name="text">
				<span class="pl-1">Run</span>
			</slot>
		{:else if category === 'edit'}
			<Icon data={faEdit} scale={0.5} />
			<slot name="text-gray-500">
				<span class="pl-1">Edit</span>
			</slot>
		{:else if category === 'archive'}
			<Icon data={faArchive} scale={0.5} />
			<slot name="text">
				<span class="pl-1">Archive</span>
			</slot>
		{:else if category === 'share'}
			<Icon data={faShare} scale={0.5} />
			<slot name="text">
				<span class="pl-1">Share</span>
			</slot>
		{/if}
	</div>
</button>
