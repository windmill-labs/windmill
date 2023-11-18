<script lang="ts">
	import { File, FolderClosed, FolderOpen, RotateCw } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService } from '$lib/gen'
	import { displayDate, displaySize, emptyString } from '$lib/utils'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import { createEventDispatcher } from 'svelte'
	import VirtualList from 'svelte-tiny-virtual-list'
	import TableSimple from './TableSimple.svelte'

	const BLURRED_FILE_KEY = 'HIDDENS3FILEKEYPATH'
	let shouldBlurFileKeys = false

	let workspaceSettingsInitialized = true

	export let initialFileKey: { s3: string }
	let initialFileKeyInternalCopy: { s3: string }
	export let selectedFileKey: { s3: string } | undefined = undefined

	let csvSeparatorDefaultChar: string = ','
	let csvSeparatorChar: string = ','

	let dispatch = createEventDispatcher()

	let drawer: Drawer
	let allFilesByKey: Record<
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
	let displayedFileKeys: string[] = []

	let listDivHeight: number = 0

	let filePreview:
		| {
				file_key: string
				mime_type: string | undefined
				size: string | undefined
				last_modified: string | undefined
				content_preview: string | undefined
				content_type: string | undefined
		  }
		| undefined = undefined

	async function loadFiles() {
		let availableFiles = await HelpersService.listStoredFiles({ workspace: $workspaceStore! })
		for (let i = 0; i < availableFiles.file_count; i++) {
			let file_path = { s3: `${BLURRED_FILE_KEY}#${i}` }
			if (availableFiles.windmill_large_files.length > i) {
				shouldBlurFileKeys = false
				file_path = availableFiles.windmill_large_files[i]
			} else {
				shouldBlurFileKeys = true
			}
			let split_path = file_path.s3.split('/')
			let parent_path: string | undefined = undefined
			let current_path: string | undefined = undefined
			let nestingLevel = 0
			for (let i = 0; i < split_path.length; i++) {
				parent_path = current_path
				current_path =
					current_path === undefined ? split_path[i] : current_path + '/' + split_path[i]

				nestingLevel = i * 2
				if (allFilesByKey[current_path] !== undefined) {
					continue
				}
				allFilesByKey[current_path] = {
					type: i === split_path.length - 1 ? 'leaf' : 'folder',
					full_key: current_path,
					display_name: split_path[i],
					collapsed: false,
					parentPath: parent_path,
					nestingLevel: nestingLevel
				}
				displayedFileKeys.push(current_path)
			}
		}
		displayedFileKeys = displayedFileKeys.sort()
	}

	async function loadFilePreview(file_key: string | undefined) {
		if (file_key === undefined) {
			return
		}
		let filePreviewRaw = await HelpersService.loadFilePreview({
			workspace: $workspaceStore!,
			fileKey: file_key,
			from: undefined,
			length: undefined,
			separator: csvSeparatorChar
		})

		if (filePreviewRaw !== undefined) {
			filePreview = {
				file_key: file_key,
				size: displaySize(filePreviewRaw.size_in_bytes),
				mime_type: filePreviewRaw.mime_type,
				last_modified: displayDate(filePreviewRaw.last_modified),
				content_preview: filePreviewRaw.content_preview.content,
				content_type: filePreviewRaw.content_preview.content_type
			}
		}
	}

	export async function open() {
		reloadContent()
		drawer.openDrawer?.()
	}

	async function reloadContent() {
		initialFileKeyInternalCopy = { ...initialFileKey }
		try {
			await HelpersService.datasetStorageTestConnection({ workspace: $workspaceStore! })
		} catch (e) {
			console.error('Workspace not connected to S3 bucket: ', e)
			workspaceSettingsInitialized = false
			return
		}
		await loadFiles() // TODO: Potentially load only on the first open and add a refresh button
		if (selectedFileKey !== undefined) {
			if (allFilesByKey[selectedFileKey.s3] === undefined) {
				selectedFileKey = { s3: '' }
			} else {
				loadFilePreview(selectedFileKey.s3)
			}
		}
	}

	async function selectAndClose() {
		drawer.closeDrawer?.()
	}

	async function exit() {
		if (initialFileKeyInternalCopy !== undefined) {
			selectedFileKey = { ...initialFileKeyInternalCopy }
		}
		drawer.closeDrawer?.()
	}

	function selectItem(index: number, toggleCollapsed: boolean = true) {
		csvSeparatorChar = csvSeparatorDefaultChar

		let item_key = displayedFileKeys[index]
		let item = allFilesByKey[item_key]
		if (item.type === 'folder') {
			if (toggleCollapsed) {
				item.collapsed = !item.collapsed
			}
			if (item.collapsed) {
				// Remove the element nested in that folder from displayed_file_keys
				let elt_to_remove = 0
				for (let i = index + 1; i < displayedFileKeys.length; i++) {
					let file_key = displayedFileKeys[i]
					if (file_key.startsWith(item_key + '/')) {
						elt_to_remove += 1
					} else {
						break
					}
				}
				if (elt_to_remove > 0) {
					displayedFileKeys.splice(index + 1, elt_to_remove)
				}
			} else {
				// Re-add the currently hidden element to displayed_file_keys
				for (let file_key in allFilesByKey) {
					let file_info = allFilesByKey[file_key]
					if (file_info.parentPath === item_key) {
						displayedFileKeys.push(file_key)
						if (file_info.type === 'folder' && !file_info.collapsed) {
							selectItem(displayedFileKeys.length - 1, false)
						}
					}
				}
			}
			displayedFileKeys = displayedFileKeys.sort()
		} else {
			selectedFileKey = {
				s3: item_key
			}
			loadFilePreview(selectedFileKey.s3)
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
		title="Pick a file"
		overflow_y={false}
		on:close={exit}
		tooltip="Files present in the Workspace S3 bucket. You can set the workspace S3 bucket in the settings."
		documentationLink="https://www.windmill.dev/docs/integrations/s3"
	>
		{#if workspaceSettingsInitialized === false}
			<Alert type="error" title="Workspace not connected to any S3 storage">
				<div class="flex flex-row gap-x-1 w-full items-center">
					<p class="text-clip grow min-w-0">
						The workspace needs to be connected to an S3 storage to use this feature. You can <a
							target="_blank"
							href="/workspace_settings?tab=windmill_lfs">configure it here</a
						>.
					</p>
					<Button
						variant="border"
						color="light"
						on:click={reloadContent}
						startIcon={{ icon: RotateCw }}
					/>
				</div>
			</Alert>
		{:else}
			{#if shouldBlurFileKeys === true}
				<Alert type="warning" title="Large buckets are only accessible with Enterprise Edition">
					<p class="text-clip grow min-w-0">
						The workspace bucket contains more than 10 files. Consider upgrading to Windmill
						Enterprise Edition to continue to use this feature.
					</p>
				</Alert>
				<div class="py-1" />
			{/if}
			<div class="flex flex-row border rounded-md h-full" bind:clientHeight={listDivHeight}>
				<div class="min-w-[30%] border-r">
					{#if displayedFileKeys.length === 0}
						<div class="p-4 text-tertiary text-xs text-center italic">
							No files in the workspace S3 bucket
						</div>
					{:else}
						<VirtualList
							width="100%"
							height={listDivHeight}
							itemCount={displayedFileKeys.length}
							itemSize={42}
						>
							<div slot="item" let:index let:style {style} class="hover:bg-surface-hover border">
								{@const file_info = allFilesByKey[displayedFileKeys[index]]}
								<div
									on:click={() => {
										if (!file_info.display_name.startsWith(BLURRED_FILE_KEY)) {
											selectItem(index)
										}
									}}
									class={`flex flex-row h-full font-semibold text-xs items-center justify-start ${
										selectedFileKey !== undefined && selectedFileKey.s3 === file_info.full_key
											? 'bg-surface-hover'
											: ''
									} `}
								>
									<div
										class={`flex flex-row w-full ml-${
											2 + file_info.nestingLevel
										} gap-2 h-full items-center ${shouldBlurFileKeys ? 'blur-sm' : ''}`}
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
					{/if}
				</div>
				<div class="flex flex-col h-full w-full overflow-auto">
					{#if filePreview === undefined}
						<div class="p-4">
							<Section label="Select a file for preview" />
						</div>
					{:else}
						<div class="p-4 gap-2">
							<Section label={filePreview.file_key} />
							<TableSimple
								headers={['Last modified', 'Size', 'Type']}
								data={[filePreview]}
								keys={['last_modified', 'size', 'mime_type']}
							/>
						</div>
					{/if}

					<div class="flex flex-col h-full w-full overflow-auto text-xs p-4 bg-surface-secondary">
						{#if filePreview !== undefined}
							<div class="flex h-6 items-center text-tertiary mb-4">
								{#if filePreview.content_type === 'Unknown'}
									Type of file not supported for preview
								{:else if filePreview.content_type === 'Csv'}
									Previewing a {filePreview.content_type?.toLowerCase()} file. Change the separator:
									<div class="inline-flex w-12 ml-2">
										<select
											class="h-8"
											bind:value={csvSeparatorChar}
											on:change={(e) => loadFilePreview(filePreview?.file_key)}
										>
											<option value=",">,</option>
											<option value=";">;</option>
										</select>
									</div>
								{:else}
									Previewing a {filePreview.content_type?.toLowerCase()} file
								{/if}
							</div>
							<pre class="grow whitespace-no-wrap break-words"
								>{#if !emptyString(filePreview.content_preview)}{filePreview.content_preview}{:else if filePreview.content_type !== undefined}Preview impossible. If it's a CSV file, you can try changing the separator{/if}</pre
							>
						{/if}
					</div>
				</div>
			</div>
		{/if}

		<div slot="actions" class="flex gap-1">
			<Button
				disable={selectedFileKey === undefined || emptyString(selectedFileKey.s3)}
				on:click={selectAndClose}>Select</Button
			>
		</div>
	</DrawerContent>
</Drawer>
