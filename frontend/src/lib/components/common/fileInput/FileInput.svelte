<script lang="ts">
	import { createEventDispatcher } from "svelte"
	import { FileUp } from "lucide-svelte"

	let c = ''
	export { c as class }
	export let accept = '*'
	export let multiple = true
	export let convertToBase64 = false
	export let hideIcon = false
	const dispatch = createEventDispatcher()
	let input: HTMLInputElement
	let files: File[] | undefined = undefined

	async function onChange(fileList: FileList | null) {
		if(!fileList || !fileList.length) {
			files = undefined;
			dispatch('change', files);
			return;
		}

		files = []
		for(let i = 0; i < fileList.length; i++) {
			const file = fileList.item(i);
			if(file) files.push(file);
		}

		if(convertToBase64) {
			const promises = files.map(fileToBase64);
			const b64s = await Promise.all(promises);
			dispatch('change', b64s);
		} else {
			dispatch('change', files);
		}
	}

	async function fileToBase64(file: File): Promise<string> {
		return new Promise((resolve) => {
			const reader = new FileReader();
			reader.onloadend = () => {
				resolve(reader.result as string);
			};
			reader.readAsDataURL(file);
		})
	}
</script>

<button
	class="relative center-center flex-col text-center font-medium text-gray-600 
	border-2 border-dashed border-gray-400 hover:border-blue-500 
	focus-within:border-blue-500 hover:bg-blue-50 focus-within:bg-blue-50 
	duration-200 p-1 {c ?? ''}"
>
	{#if !hideIcon && !files}
		<FileUp size={36} class="mb-2" />
	{/if}
	{#if files}
		<div class="text-center">
			Selected file{files.length > 1 ? 's' : ''}:
			<ul>
				{#each files as {name}}
					<li class="font-normal text-sm">
						{name}
					</li>
				{/each}
			</ul>
		</div>
	{:else}
		<slot>
			<span>Drag and drop files</span>
		</slot>
	{/if}
	<input
		class="!absolute !inset-0 !opacity-0"
		type="file"
		bind:this={input}
		on:change={({currentTarget}) => {onChange(currentTarget.files)}}
		{accept}
		{multiple}
		{...$$restProps}
	/>
</button>