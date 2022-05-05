<script lang="ts">
	import {
		faArchive,
		faEdit,
		faList,
		faPlay,
		faShare,
		faTrash
	} from '@fortawesome/free-solid-svg-icons';
	import { createEventDispatcher } from 'svelte';
	import Icon from 'svelte-awesome';

	export let category: 'delete' | 'list' | 'run' | 'add' | 'edit' | 'archive' | 'share';
	export let disabled: boolean = false;
	const dispatch = createEventDispatcher();
</script>

<button
	class="{$$props.class} inline-flex items-center default-button py-0 px-1 {category} default-button-secondary"
	on:click={() => {
		dispatch('click');
	}}
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

<style>
	.delete,
	.archive {
		@apply bg-transparent hover:bg-red-600;
		@apply text-red-600 font-normal hover:text-white;
		@apply border-red-600 hover:border-transparent rounded;
	}

	.edit,
	.play,
	.list,
	.share {
		@apply hover:bg-blue-500 hover:text-white;
	}
</style>
