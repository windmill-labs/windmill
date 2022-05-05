<script lang="ts">
	import { faSortDown } from '@fortawesome/free-solid-svg-icons';
	import type { DropdownItem } from '../../utils';
	import { createEventDispatcher } from 'svelte';
	import Icon from 'svelte-awesome';
	import Dropdown from './Dropdown.svelte';

	export let dropdownItems: DropdownItem[] = [];

	const dispatch = createEventDispatcher();
</script>

<div class="bg-blue-500 rounded-sm {$$props.class} inline-block py-0 my-0 ">
	<button
		class="inline pl-2 text-white text-sm py-0 my-0"
		on:click={() => {
			dispatch('clickMain');
		}}><slot name="name">Add script</slot></button
	>
	<Dropdown
		{dropdownItems}
		class="border-l border-gray-300 px-2 inline-block py-0 my-0"
		on:click={() => dispatch('clickDropdown')}
	>
		<div slot="name" class="py-0 my-0">
			<Icon class="text-white align-top py-0 my-0" data={faSortDown} />
		</div>
	</Dropdown>
</div>
