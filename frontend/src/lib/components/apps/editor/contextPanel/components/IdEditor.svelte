<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { allItems } from '$lib/components/apps/utils'
	import { forbiddenIds } from '$lib/components/flows/idUtils'
	import { ArrowRight, Pencil } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fade, slide } from 'svelte/transition'
	import { Button, Popup } from '../../../../common'

	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	export let id: string
	const dispatch = createEventDispatcher()
	const regex = /^[a-zA-Z0-9]{1,}$/
	let value = id
	let button: HTMLButtonElement
	let input: HTMLInputElement
	let error = ''

	$: if (!regex.test(value)) {
		error = 'The ID must include only letters and numbers'
	} else if (forbiddenIds.includes(value)) {
		error = 'This ID is reserved'
	} else if (
		allItems($app.grid, $app.subgrids).some((item) => item.id === value && item.id !== id)
	) {
		error = 'This ID is already in use'
	} else {
		id = value
		error = ''
	}

	function save() {
		if (error != '') {
			return
		}
		dispatch('change', id)
		input.blur()
	}
</script>

<button
	on:click|stopPropagation={() => {
		$selectedComponent = [id]
	}}
	bind:this={button}
	title="Edit ID"
	class="flex items-center px-1 rounded-sm bg-gray-100 hover:text-black text-gray-600"
	aria-label="Open component ID editor"
>
	<Pencil size={14} />
</button>
<Popup
	ref={button}
	options={{ placement: 'top-start' }}
	transition={fade}
	wrapperClasses="!z-[1002]"
	outerClasses="rounded shadow-xl bg-white border p-3"
	on:close={() => (value = id)}
>
	<label class="block text-gray-900">
		<div class="pb-1 text-sm text-gray-600">Component ID</div>
		<div class="flex w-full">
			<input
				type="text"
				bind:value
				class="!w-auto grow"
				bind:this={input}
				on:click|stopPropagation={() => {}}
				on:keypress={({ key }) => {
					if (key === 'Enter') save()
				}}
			/>
			<Button
				size="xs"
				color="blue"
				buttonType="button"
				btnClasses="!p-1 !w-[34px] !ml-1"
				aria-label="Save ID"
				disabled={error != ''}
				on:click={save}
			>
				<ArrowRight size={18} />
			</Button>
		</div>
		{#if error != ''}
			<div
				transition:slide|local={{ duration: 100 }}
				class="w-full text-sm text-red-600 whitespace-pre-wrap pt-1"
			>
				{error}
			</div>
		{/if}
	</label>
</Popup>
