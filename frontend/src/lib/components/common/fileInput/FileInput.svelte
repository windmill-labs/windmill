<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { FileUp, Trash } from 'lucide-svelte'
	import Button from '../../common/button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ReadFileAs } from './model'

	type ConvertedFile = string | ArrayBuffer | null

	let c = ''
	export { c as class }
	export let style = ''
	export let accept = '*'
	export let multiple = false
	export let convertTo: ReadFileAs | undefined = undefined
	export let hideIcon = false
	export let iconSize = 36
	export let returnFileNames = false
	const dispatch = createEventDispatcher()
	let input: HTMLInputElement
	let files: File[] | undefined = undefined

	async function onChange(fileList: FileList | null) {
		if (!fileList || !fileList.length) {
			files = undefined
			dispatch('change', files)
			return
		}

		if (!multiple || !files) {
			files = []
		}
		for (let i = 0; i < fileList.length; i++) {
			const file = fileList.item(i)
			if (file) files.push(file)
		}
		// Needs to be reset so the same file can be selected
		// multiple times in a row
		input.value = ''

		dispatchChange()
	}

	async function convertFile(file: File): Promise<string | ArrayBuffer | null> {
		return new Promise((resolve) => {
			if (!convertTo) {
				return resolve(null)
			}
			const reader = new FileReader()
			reader.onloadend = () => {
				resolve(reader.result)
			}
			switch (convertTo) {
				case 'buffer':
					reader.readAsArrayBuffer(file)
					break
				case 'binary':
					reader.readAsBinaryString(file)
					break
				case 'base64':
					reader.readAsDataURL(file)
					break
				case 'text':
					reader.readAsText(file)
					break
			}
		})
	}

	function removeFile(index: number) {
		if (!files) return
		files.splice(index, 1)
		files = files.length ? files : undefined
		dispatchChange()
	}

	async function dispatchChange() {
		files = files

		if (convertTo && files) {
			const promises = files.map(convertFile)
			let converted: ConvertedFile[] | { name: string; data: ConvertedFile }[] = await Promise.all(
				promises
			)
			if (returnFileNames) {
				converted = converted.map((c, i) => ({ name: files![i].name, data: c }))
			}
			dispatch('change', converted)
		} else {
			dispatch('change', files)
		}
	}
</script>

<button
	class={twMerge(
		`relative center-center flex-col text-center font-medium text-tertiary 
		border-2 border-dashed border-gray-400 hover:border-blue-500 
		focus-within:border-blue-500 hover:bg-blue-50 dark:hover:bg-frost-900 focus-within:bg-blue-50 
		duration-200 rounded-lg p-1`,
		c
	)}
	{style}
>
	{#if !hideIcon && !files}
		<FileUp size={iconSize} class="mb-2" />
	{/if}
	{#if files}
		<div class="w-full max-h-full overflow-auto px-6">
			<slot name="selected-title">
				<div class="text-center mb-2 px-2">
					Selected file{files.length > 1 ? 's' : ''}:
				</div>
			</slot>
			<ul class="relative z-20 max-w-[500px] bg-surface rounded-lg overflow-hidden mx-auto">
				{#each files as { name }, i}
					<li
						class="flex justify-between items-center font-normal text-sm
					hover:bg-gray-300/20 duration-200 py-1 px-2 cursor-default"
					>
						<span class="pr-2 ellipsize">{name}</span>
						<Button
							size="xs"
							color="red"
							variant="border"
							iconOnly
							btnClasses="bg-transparent"
							startIcon={{ icon: Trash }}
							on:click={() => removeFile(i)}
						/>
					</li>
				{/each}
			</ul>
		</div>
	{:else}
		<slot>
			<span>Drag and drop {multiple ? 'files' : 'a file'}</span>
		</slot>
	{/if}
	<input
		class="!absolute !inset-0 !z-10 !opacity-0 !cursor-pointer"
		type="file"
		title={files ? `${files.length} file${files.length > 1 ? 's' : ''} chosen` : 'No file chosen'}
		bind:this={input}
		on:change={({ currentTarget }) => {
			onChange(currentTarget.files)
		}}
		{accept}
		{multiple}
		{...$$restProps}
	/>
</button>
