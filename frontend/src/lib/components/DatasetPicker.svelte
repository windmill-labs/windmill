<script lang="ts">
	import { File, FolderClosed, FolderOpen } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService } from '$lib/gen'
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { createEventDispatcher } from 'svelte'
	import VirtualList from 'svelte-tiny-virtual-list'

	export let pickedDatasetKey: { s3: string } | undefined = undefined

	let dispatch = createEventDispatcher()

	let drawer: Drawer
	let all_files_by_key: Record<
		string,
		{
			type: 'folder' | 'leaf'
			full_key: string
			display_name: string
			collapsed: boolean
			parentPath: string | undefined
			nestingLevel: number
		}
	> = {}
	let displayed_file_keys: string[] = []

	let listDivHeight: number = 0

	let filePreview:
		| {
				file_key: string
				last_modified: string | undefined
				content_preview: string | undefined
		  }
		| undefined = undefined

	async function loadDatasets() {
		let availableFiles = await HelpersService.listStoredDatasets({ workspace: $workspaceStore! })
		if (
			availableFiles.windmill_large_files !== undefined &&
			availableFiles.windmill_large_files.length > 0
		) {
			let file_paths = availableFiles.windmill_large_files
			for (let file_path of file_paths) {
				let split_path = file_path.s3.split('/')
				let parent_path: string | undefined = undefined
				let current_path: string | undefined = undefined
				let nestingLevel = 0
				for (let i = 0; i < split_path.length; i++) {
					parent_path = current_path
					current_path =
						current_path === undefined ? split_path[i] : current_path + '/' + split_path[i]

					nestingLevel = i * 2
					if (all_files_by_key[current_path] !== undefined) {
						continue
					}
					console.log(current_path)
					all_files_by_key[current_path] = {
						type: i === split_path.length - 1 ? 'leaf' : 'folder',
						full_key: current_path,
						display_name: split_path[i],
						collapsed: false,
						parentPath: parent_path,
						nestingLevel: nestingLevel
					}
					displayed_file_keys.push(current_path)
				}
			}
			displayed_file_keys = displayed_file_keys.sort()
			console.log(displayed_file_keys)
			console.log(all_files_by_key)
		}
	}

	async function loadFilePreview(file_key: string) {
		let filePreviewRaw = await HelpersService.loadFilePreview({
			workspace: $workspaceStore!,
			fileKey: file_key,
			from: undefined,
			length: undefined,
			separator: undefined
		})
		if (filePreviewRaw !== undefined) {
			filePreview = {
				file_key: file_key,
				last_modified: filePreviewRaw.last_modified,
				content_preview: filePreviewRaw.content_preview
			}
		}
	}

	export async function open() {
		await loadDatasets() // TODO: Potentially load only on the first open and add a refresh button
		drawer.openDrawer?.()
	}

	async function close() {
		drawer.closeDrawer?.()
	}

	function selectItem(index: number, toggleCollapsed: boolean = true) {
		let item_key = displayed_file_keys[index]
		let item = all_files_by_key[item_key]
		if (item.type === 'folder') {
			if (toggleCollapsed) {
				item.collapsed = !item.collapsed
			}
			if (item.collapsed) {
				// Remove the element nested in that folder from displayed_file_keys
				let elt_to_remove = 0
				for (let i = index + 1; i < displayed_file_keys.length; i++) {
					let file_key = displayed_file_keys[i]
					if (file_key.startsWith(item_key + '/')) {
						elt_to_remove += 1
					} else {
						break
					}
				}
				if (elt_to_remove > 0) {
					displayed_file_keys.splice(index + 1, elt_to_remove)
				}
			} else {
				// Re-add the currently hidden element to displayed_file_keys
				for (let file_key in all_files_by_key) {
					let file_info = all_files_by_key[file_key]
					if (file_info.parentPath === item_key) {
						displayed_file_keys.push(file_key)
						if (file_info.type === 'folder' && !file_info.collapsed) {
							selectItem(displayed_file_keys.length - 1, false)
						}
					}
				}
			}
			displayed_file_keys = displayed_file_keys.sort()
		} else {
			pickedDatasetKey = {
				s3: item_key
			}
			loadFilePreview(item_key)
		}
	}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<Drawer
	bind:this={drawer}
	on:close={() => {
		dispatch('close')
	}}
	size="1200px"
>
	<DrawerContent
		title="Pick a dataset"
		overflow_y={false}
		on:close={drawer.closeDrawer}
		tooltip="Datasets present in the Workspace S3 bucket. You can set the workspace S3 bucket in the settings."
		documentationLink="https://www.windmill.dev/docs/integrations/s3"
	>
		<div class="flex flex-row border rounded-md h-full" bind:clientHeight={listDivHeight}>
			<div class="min-w-[30%]">
				<VirtualList
					width="100%"
					height={listDivHeight}
					itemCount={displayed_file_keys.length}
					itemSize={42}
				>
					<div slot="item" let:index let:style {style} class="hover:bg-surface-hover border">
						{@const file_info = all_files_by_key[displayed_file_keys[index]]}
						<div
							on:click={() => selectItem(index)}
							class={`flex flex-row h-full font-semibold text-xs items-center justify-start ${
								pickedDatasetKey !== undefined && pickedDatasetKey.s3 === file_info.full_key
									? 'bg-surface-hover'
									: ''
							}`}
						>
							<div
								class={`flex flex-row w-full ml-${
									2 + file_info.nestingLevel
								} gap-2 h-full items-center`}
							>
								{#if file_info.type === 'folder'}
									{#if file_info.collapsed}<FolderClosed />{:else}<FolderOpen />{/if}
									{file_info.display_name}
								{:else}
									<File />
									{file_info.display_name}
								{/if}
							</div>
						</div>
					</div></VirtualList
				>
			</div>
			<div class="flex h-full w-full overflow-auto">
				<pre class="grow whitespace-no-wrap break-words bg-surface-secondary text-xs p-2"
					>{#if filePreview !== undefined}{filePreview.content_preview}{/if}
                </pre>
			</div>
		</div>

		<div slot="actions" class="flex gap-1">
			<Button disable={pickedDatasetKey === undefined} on:click={close}>Select</Button>
		</div>
	</DrawerContent>
</Drawer>
