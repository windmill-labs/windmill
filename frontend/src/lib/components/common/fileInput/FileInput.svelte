<script lang="ts">
	import { createEventDispatcher } from "svelte"
	import { FileUp } from "lucide-svelte"
	import Button from "../../common/button/Button.svelte"
	import { faTrash } from "@fortawesome/free-solid-svg-icons"

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

		if(!multiple || !files) {
			files = []
		}
		for(let i = 0; i < fileList.length; i++) {
			const file = fileList.item(i);
			if(file) files.push(file);
		}
		// Needs to be reset so the same file can be selected 
		// multiple times in a row
		input.value = ''

		dispatchChange()
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

	function removeFile(index: number) {
		if(!files) return;
		files.splice(index, 1)
		files = files.length ? files : undefined
		dispatchChange()
	}

	async function dispatchChange() {
		files = files

		if(convertToBase64 && files) {
			const promises = files.map(fileToBase64);
			const b64s = await Promise.all(promises);
			dispatch('change', b64s);
		} else {
			dispatch('change', files);
		}
	}
</script>

<button
	class="relative center-center flex-col text-center font-medium text-gray-600 
	border-2 border-dashed border-gray-400 hover:border-blue-500 
	focus-within:border-blue-500 hover:bg-blue-50 focus-within:bg-blue-50 
	duration-200 rounded-lg p-1 {c ?? ''}"
>
	{#if !hideIcon && !files}
		<FileUp size={36} class="mb-2" />
	{/if}
	{#if files}
		<div class="w-full max-h-full overflow-auto px-6">
			<div class="text-center mb-2 px-2">Selected file{files.length > 1 ? 's' : ''}:</div>
			<ul class="relative z-20 max-w-[250px] bg-white rounded-lg overflow-hidden mx-auto">
				{#each files as {name}, i}
					<li class="flex justify-between items-center font-normal text-sm
					hover:bg-gray-300/20 duration-200 py-1 px-2 cursor-default">
						<span class="pr-2 ellipsize">{name}</span>
						<Button
							size="xs"
							color="red"
							variant="border"
							iconOnly
							btnClasses="bg-transparent"
							startIcon={{ icon: faTrash }}
							on:click={() => removeFile(i)}
						/>
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
		class="!absolute !inset-0 !z-10 !opacity-0 !cursor-pointer"
		type="file"
		title={files ? `${files.length} file${files.length > 1 ? 's' : ''} chosen` : 'No file chosen'}
		bind:this={input}
		on:change={({currentTarget}) => {onChange(currentTarget.files)}}
		{accept}
		{multiple}
		{...$$restProps}
	/>
</button>