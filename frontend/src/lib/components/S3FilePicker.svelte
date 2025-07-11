<script lang="ts">
	import { run, createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import {
		File as FileIcon,
		FolderClosed,
		FolderOpen,
		RotateCw,
		Loader2,
		Download,
		Trash,
		FileUp,
		MoveRight
	} from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService, SettingService } from '$lib/gen'
	import { base } from '$lib/base'
	import {
		displayDate,
		displaySize,
		emptyString,
		parseS3Object,
		sendUserToast,
		type S3Object
	} from '$lib/utils'
	import { Alert, Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import Section from './Section.svelte'
	import { createEventDispatcher, untrack } from 'svelte'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import TableSimple from './TableSimple.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import FileUploadModal from './common/fileUpload/FileUploadModal.svelte'
	import { twMerge } from 'tailwind-merge'
	import Select from './select/Select.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'

	let deletionModalOpen = $state(false)
	let fileDeletionInProgress = $state(false)

	let fileListUnavailable: boolean | undefined = $state(undefined)

	let moveModalOpen = $state(false)
	let moveDestKey: string | undefined = $state(undefined)
	let fileMoveInProgress = $state(false)

	let uploadModalOpen = $state(false)

	let workspaceSettingsInitialized = $state(true)

	let initialFileKeyInternalCopy: { s3: string; storage?: string }
	interface Props {
		fromWorkspaceSettings?: boolean
		readOnlyMode: boolean
		initialFileKey?: { s3: string; storage?: string } | undefined
		selectedFileKey?: { s3: string; storage?: string } | undefined
		folderOnly?: boolean
		regexFilter?: RegExp | undefined
	}

	let {
		fromWorkspaceSettings = false,
		readOnlyMode,
		initialFileKey = $bindable(undefined),
		selectedFileKey = $bindable(undefined),
		folderOnly = false,
		regexFilter = undefined
	}: Props = $props()

	let csvSeparatorChar: string = $state(',')
	let csvHasHeader: boolean = $state(true)

	let dispatch = createEventDispatcher<{
		close: { s3: string; storage: string | undefined } | undefined
		selectAndClose: { s3: string; storage: string | undefined }
	}>()

	let drawer: Drawer | undefined = $state()

	let fileInfoLoading: boolean = $state(true)
	let fileListLoading: boolean = $state(true)
	let allFilesByKey: Record<
		string,
		{
			type: 'folder' | 'leaf'
			full_key: string
			display_name: string
			collapsed: boolean
			parentPath: string | undefined
			nestingLevel: number
			count: number
		}
	> = $state({})
	let displayedFileKeys: string[] = $state([])

	let listDivHeight: number = $state(0)

	let fileMetadata:
		| {
				fileKey: string
				mimeType: string | undefined
				size: number | undefined
				sizeStr: string | undefined
				lastModified: string | undefined
		  }
		| undefined = $state(undefined)
	let filePreviewLoading: boolean = $state(false)
	let filePreview:
		| {
				fileKey: string
				contentPreview: string | undefined
				contentType: string | undefined
		  }
		| undefined = $state(undefined)

	let listMarkers: string[]
	let page = $state(0)

	const maxKeys = 1000

	let count = $state(0)
	let displayedCount = $state(0)

	let filter = $state('')

	let timeout: NodeJS.Timeout | undefined = undefined
	let firstLoad = true

	let secondaryStorageNames = usePromise(
		() => SettingService.getSecondaryStorageNames({ workspace: $workspaceStore! }),
		{ loadInit: false }
	)
	$effect(() => {
		$workspaceStore && untrack(() => secondaryStorageNames.refresh())
	})

	function onFilterChange() {
		if (!firstLoad) {
			timeout && clearTimeout(timeout)
			timeout = setTimeout(() => {
				clearAndLoadFiles({ keepFilter: true })
			}, 500)
		} else {
			firstLoad = false
		}
	}

	let lastKeyFolders: string[] = $state([])
	async function loadFiles() {
		fileListLoading = true
		let availableFiles = await HelpersService.listStoredFiles({
			workspace: $workspaceStore!,
			maxKeys: maxKeys, // fixed pages of 1000 files for now
			marker: page == 0 ? undefined : listMarkers[page - 1],
			prefix: filter.trim() != '' ? filter : undefined,
			storage: storage
		})
		if (
			availableFiles.restricted_access === null ||
			availableFiles.restricted_access === undefined ||
			availableFiles.restricted_access === true
		) {
			fileListUnavailable = true
			loadFileMetadataPlusPreviewAsync(selectedFileKey?.s3)
			return
		}
		fileListUnavailable = false
		for (let [index, file_path] of availableFiles.windmill_large_files.entries()) {
			if (regexFilter && !regexFilter.test(file_path.s3)) {
				continue
			}
			displayedCount += 1
			let split_path = file_path.s3.split('/')
			let parent_path: string | undefined = undefined
			let current_path: string | undefined = undefined
			let nestingLevel = 0

			if (index === availableFiles.windmill_large_files.length - 1 && split_path.length > 1) {
				lastKeyFolders = split_path.slice(0, -1)
			}

			for (let i = 0; i < split_path.length; i++) {
				parent_path = current_path
				current_path = current_path === undefined ? split_path[i] : current_path + split_path[i]

				if (i < split_path.length - 1) {
					current_path += '/'
				}

				nestingLevel = i * 2
				if (allFilesByKey[current_path] !== undefined) {
					allFilesByKey[current_path].count += 1
					continue
				}
				allFilesByKey[current_path] = {
					type: i === split_path.length - 1 ? 'leaf' : 'folder',
					full_key: current_path,
					display_name: split_path[i],
					collapsed: true, // folders collapsed by default
					parentPath: parent_path,
					nestingLevel: nestingLevel,
					count: 1
				}
				if (i == 0) {
					displayedFileKeys.push(current_path)
				}
			}
		}
		if (listMarkers.length == page) {
			count += availableFiles.windmill_large_files.length
			const nextMarker =
				availableFiles.windmill_large_files?.[availableFiles.windmill_large_files.length - 1]?.s3
			if (nextMarker) listMarkers.push(nextMarker)
		}

		// before returning, un-collapse the folders containing the selected file (if any)
		if (selectedFileKey !== undefined && !emptyString(selectedFileKey.s3) && page === 0) {
			let split_path = selectedFileKey.s3.split('/')
			let current_path: string | undefined = undefined
			for (let i = 0; i < split_path.length; i++) {
				current_path = current_path === undefined ? split_path[i] : current_path + split_path[i]
				if (i < split_path.length - 1) {
					current_path += '/'
				}
				const folder = allFilesByKey[current_path]
				if (folder) {
					folder.collapsed = false
				}
				for (let file_key in allFilesByKey) {
					let file_info = allFilesByKey[file_key]
					if (file_info.parentPath === current_path) {
						displayedFileKeys.push(file_key)
					}
				}
			}
		}
		displayedFileKeys = displayedFileKeys.sort()
		fileListLoading = false
		fileInfoLoading = false
	}

	async function loadFileMetadataPlusPreviewAsync(fileKey: string | undefined) {
		if (fileKey === undefined || emptyString(fileKey)) {
			fileInfoLoading = false
			return
		}
		fileInfoLoading = true
		let fileMetadataRaw = await HelpersService.loadFileMetadata({
			workspace: $workspaceStore!,
			fileKey: fileKey,
			storage: storage
		})

		if (fileMetadataRaw !== undefined) {
			fileMetadata = {
				fileKey: fileKey,
				size: fileMetadataRaw.size_in_bytes,
				sizeStr: displaySize(fileMetadataRaw.size_in_bytes),
				mimeType: fileMetadataRaw.mime_type,
				lastModified: displayDate(fileMetadataRaw.last_modified)
			}
		}
		// async call
		loadFilePreview(fileKey, fileMetadataRaw.size_in_bytes, fileMetadataRaw.mime_type)
	}

	async function loadFilePreview(fileKey: string, fileSizeInBytes?: number, fileMimeType?: string) {
		filePreviewLoading = true
		let filePreviewRaw = await HelpersService.loadFilePreview({
			workspace: $workspaceStore!,
			fileKey: fileKey,
			fileSizeInBytes: fileSizeInBytes,
			fileMimeType: fileMimeType,
			csvSeparator: csvSeparatorChar,
			csvHasHeader: csvHasHeader,
			readBytesFrom: 0,
			readBytesLength: 128 * 1024, // For now static limit of 128Kb per file,
			storage: storage
		})

		let filePreviewContent = filePreviewRaw.content
		if (
			filePreviewContent !== null &&
			filePreviewContent !== undefined &&
			filePreviewContent.length >= 128 * 1024
		) {
			filePreviewContent =
				filePreviewContent?.substring(0, 128 * 1024 - 35) +
				'\n\n ... FILE CONTENT TRUNCATED ...\n\n'
		}

		if (filePreviewRaw !== undefined) {
			filePreview = {
				fileKey: fileKey,
				contentPreview: filePreviewContent,
				contentType: filePreviewRaw.content_type
			}
			if (fileMetadata) {
				fileMetadata.mimeType =
					((fileKey.endsWith('.png') ||
						fileKey.endsWith('.jpg') ||
						fileKey.endsWith('.jpeg') ||
						fileKey.endsWith('.webp')) &&
						'Image') ||
					(fileKey.endsWith('.pdf') && 'PDF') ||
					filePreview.contentType
			}
		}
		filePreviewLoading = false
		fileInfoLoading = false
	}

	async function deleteFileFromS3(fileKey: string | undefined) {
		fileDeletionInProgress = true
		if (fileKey === undefined) {
			return
		}
		try {
			await HelpersService.deleteS3File({
				workspace: $workspaceStore!,
				fileKey: fileKey,
				storage: storage
			})
		} finally {
			fileDeletionInProgress = false
			deletionModalOpen = false
		}
		sendUserToast(`${fileKey} deleted from S3 bucket`)
		selectedFileKey = { s3: '', storage }
		const currentPage = page
		await clearAndLoadFiles()
		for (let i = 0; i < currentPage; i++) {
			page = i + 1
			await loadFiles()
		}
		const fileKeyFolders = fileKey.split('/').slice(0, -1)
		let current_path: string | undefined = undefined
		for (let i = 0; i < fileKeyFolders.length; i++) {
			current_path =
				current_path === undefined ? fileKeyFolders[i] : current_path + fileKeyFolders[i]
			if (i < fileKeyFolders.length) {
				current_path += '/'
			}
			const folder = allFilesByKey[current_path]
			if (folder) {
				folder.collapsed = false
			}
			for (let file_key in allFilesByKey) {
				let file_info = allFilesByKey[file_key]
				if (file_info.parentPath === current_path) {
					displayedFileKeys.push(file_key)
				}
			}
		}
		displayedFileKeys = displayedFileKeys.sort()
	}

	async function clearAndLoadFiles({ keepFilter }: { keepFilter?: boolean } = {}) {
		displayedFileKeys = []
		allFilesByKey = {}
		count = 0
		displayedCount = 0
		page = 0
		listMarkers = []
		fileMetadata = undefined
		filePreview = undefined
		if (!keepFilter) {
			filter = ''
		}
		await loadFiles()
	}

	async function moveS3File(srcFileKey: string | undefined, destFileKey: string | undefined) {
		fileMoveInProgress = true
		if (srcFileKey === undefined || emptyString(destFileKey)) {
			return
		}
		try {
			await HelpersService.moveS3File({
				workspace: $workspaceStore!,
				srcFileKey: srcFileKey,
				destFileKey: destFileKey!,
				storage: storage
			})
		} finally {
			fileMoveInProgress = false
			moveModalOpen = false
		}
		sendUserToast(`${srcFileKey} moved to ${destFileKey}`)
		selectedFileKey = { s3: destFileKey!, storage }
		await clearAndLoadFiles()
		await loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
	}

	let storage: string | undefined = $state(undefined)
	export async function open(_preSelectedFileKey: S3Object | undefined = undefined) {
		const preSelectedFileKey = _preSelectedFileKey && parseS3Object(_preSelectedFileKey)
		storage = preSelectedFileKey?.storage
		if (preSelectedFileKey !== undefined) {
			initialFileKey = { ...preSelectedFileKey }
			selectedFileKey = { ...preSelectedFileKey }
		}
		reloadContent()
		drawer?.openDrawer?.()
	}

	async function reloadContent() {
		if (initialFileKey !== undefined) {
			initialFileKeyInternalCopy = { ...initialFileKey }
		}
		fileListLoading = true
		try {
			await HelpersService.datasetStorageTestConnection({
				workspace: $workspaceStore!,
				storage: storage
			})
			workspaceSettingsInitialized = true
		} catch (e) {
			fileListLoading = false
			console.error('Workspace not connected to object storage: ', e)
			workspaceSettingsInitialized = false
			return
		}
		await clearAndLoadFiles()
		if (selectedFileKey !== undefined) {
			if (allFilesByKey[selectedFileKey.s3] === undefined) {
				selectedFileKey = { s3: '', storage }
			} else {
				loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
			}
		}
	}

	async function selectAndClose() {
		if (selectedFileKey?.s3) {
			dispatch('selectAndClose', { s3: selectedFileKey.s3, storage })
		}
		drawer?.closeDrawer?.()
	}

	async function exit() {
		if (initialFileKeyInternalCopy !== undefined) {
			selectedFileKey = { ...initialFileKeyInternalCopy }
		}
		drawer?.closeDrawer?.()
	}

	function selectItem(index: number, toggleCollapsed: boolean = true) {
		let item_key = displayedFileKeys[index]
		let item = allFilesByKey[item_key]
		if (item.type === 'folder') {
			if (folderOnly) {
				selectedFileKey = {
					s3: item_key,
					storage
				}
			}
			if (toggleCollapsed) {
				item.collapsed = !item.collapsed
			}
			if (item.collapsed) {
				// Remove the element nested in that folder from displayed_file_keys
				let elt_to_remove = 0
				for (let i = index + 1; i < displayedFileKeys.length; i++) {
					let file_key = displayedFileKeys[i]
					if (file_key.startsWith(item_key)) {
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
				s3: item_key,
				storage
			}
			loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
		}
	}
	run(() => {
		filter != undefined && untrack(() => onFilterChange())
	})
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<Drawer
	bind:this={drawer}
	on:close={() => {
		dispatch(
			'close',
			selectedFileKey?.s3
				? {
						s3: selectedFileKey.s3,
						storage: storage
					}
				: undefined
		)
	}}
	size="1200px"
>
	<DrawerContent
		title="S3 file browser"
		on:close={exit}
		tooltip="Files present in the Workspace S3 bucket. You can set the workspace S3 bucket in the settings."
		documentationLink="https://www.windmill.dev/docs/integrations/s3"
	>
		{#if workspaceSettingsInitialized === false}
			{#if fromWorkspaceSettings}
				<Alert type="error" title="Connection to remote S3 bucket unsuccessful">
					<div class="flex flex-row gap-x-1 w-full items-center">
						<p class="text-clip grow min-w-0">
							Double check the S3 resource fields and try again.
						</p>
					</div>
				</Alert>
			{:else}
				<Alert type="error" title="Workspace not connected to any S3 storage">
					<div class="flex flex-row gap-x-1 w-full items-center">
						<p class="text-clip grow min-w-0">
							The workspace needs to be connected to an S3 storage to use this feature. You can <a
								target="_blank"
								href="{base}/workspace_settings?tab=windmill_lfs">configure it here</a
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
			{/if}
		{:else}
			{#if fileListUnavailable == true}
				<div class="mb-2">
					<Alert type="info" title="Access to S3 bucket restricted">
						<p>
							You don't have access to the S3 bucket resource and your administrator has restricted
							the access to it. You are not authorized to browse the bucket content. If you think
							this is incorrect, please contact your workspace administrator.
						</p>
						<p>
							More info in <a
								href="https://www.windmill.dev/docs/core_concepts/persistent_storage/large_data_files"
								target="_blank">Windmill's documentation</a
							></p
						></Alert
					>
				</div>
			{/if}
			<div class="flex flex-row border rounded-md h-full">
				{#if !fileListUnavailable}
					<div class="min-w-[30%] border-r flex flex-col">
						<div class="w-full p-1 border-b">
							<input type="text" placeholder="Folder prefix" bind:value={filter} class="text-xl" />
						</div>
						{#if fileListLoading === false && displayedFileKeys.length === 0}
							<div class="p-4 text-tertiary text-xs text-center italic">
								No files in the workspace S3 bucket at that prefix
							</div>
						{:else}
							<div class="grow" bind:clientHeight={listDivHeight}>
								<VirtualList
									width="100%"
									height={listDivHeight}
									itemCount={displayedFileKeys.length}
									itemSize={42}
								>
									{#snippet header()}{/snippet}
									{#snippet footer()}{/snippet}
									{#snippet children({ index, style })}
										{@const file_info = allFilesByKey[displayedFileKeys[index]]}

										<div
											{style}
											class={twMerge(
												'hover:bg-surface-hover border-b',
												index === displayedFileKeys.length - 1 && 'border-b-0'
											)}
										>
											{#if file_info}
												<div
													onclick={() => selectItem(index)}
													class={twMerge(
														'flex flex-row h-full font-semibold text-xs items-center justify-start',
														selectedFileKey !== undefined &&
															selectedFileKey.s3 === file_info.full_key
															? 'bg-surface-hover'
															: ''
													)}
												>
													<div
														class={`flex flex-row w-full gap-2 h-full items-center`}
														style={`margin-left: ${(2 + file_info.nestingLevel) * 0.25}rem;`}
													>
														{#if file_info.type === 'folder'}
															{#if file_info.collapsed}<FolderClosed size={16} />{:else}<FolderOpen
																	size={16}
																/>{/if}
															<div class="truncate text-ellipsis w-56">
																{file_info.display_name} ({file_info.count}{count % 1000 === 0 &&
																lastKeyFolders[file_info.nestingLevel / 2] ===
																	file_info.display_name
																	? '+'
																	: ''} item{file_info.count === 1 ? '' : 's'})
															</div>
														{:else}
															<FileIcon size={16} />
															<div class="truncate text-ellipsis w-56">
																{file_info.display_name}
															</div>
														{/if}
													</div>
												</div>
											{/if}
										</div>
									{/snippet}
								</VirtualList>
							</div>
							<div
								class="flex flex-col gap-2 text-2xs justify-center items-center text-secondary w-full border-t h-16"
							>
								{#if fileListLoading === true}
									<div class="flex text-secondary mt-1 text-xs justify-center items-center w-full">
										<Loader2 size={12} class="animate-spin mr-1" /> Loading content
									</div>
								{:else}
									<div>
										{displayedCount}{count % maxKeys === 0 ? '+' : ''}
										{displayedCount !== count ? 'filtered ' : ''}items (including inside folders)
									</div>

									{#if count % maxKeys === 0}
										<Button
											variant="border"
											color="light"
											size="xs2"
											on:click={() => {
												page += 1
												loadFiles()
											}}
										>
											Load more
										</Button>
									{/if}
								{/if}
							</div>
						{/if}
					</div>
				{/if}
				<div class="flex flex-col h-full w-full overflow-auto">
					{#if fileMetadata === undefined}
						<div class="p-4">
							{#if fileInfoLoading}
								<Section label="Loading..." />
							{:else if fileListUnavailable}
								<Section label="No file to preview" />
							{:else}
								<Section label="Select a file to preview" />
							{/if}
						</div>
					{:else}
						<div class="p-4 gap-2">
							<Section label={fileMetadata.fileKey} breakAll>
								{#snippet action()}
									<div class="flex gap-2">
										{#if filePreview !== undefined}
											<Button
												title="Download file from S3"
												variant="border"
												color="light"
												href={`${base}/api/w/${$workspaceStore}/job_helpers/download_s3_file?file_key=${encodeURIComponent(
													fileMetadata?.fileKey ?? ''
												)}${storage ? `&storage=${storage}` : ''}`}
												download={fileMetadata?.fileKey.split('/').pop() ?? 'unnamed_download.file'}
												startIcon={{ icon: Download }}
												iconOnly={true}
											/>
											<Button
												title="Move file"
												variant="border"
												color="light"
												on:click={() => {
													moveDestKey = fileMetadata?.fileKey ?? ''
													moveModalOpen = true
												}}
												startIcon={{ icon: MoveRight }}
												iconOnly={true}
											/>
											<Button
												title="Delete file from S3"
												variant="border"
												color="red"
												on:click={() => {
													deletionModalOpen = true
												}}
												startIcon={{ icon: Trash }}
												iconOnly={true}
											/>
										{/if}
									</div>
								{/snippet}
							</Section>
							<TableSimple
								headers={['Last modified', 'Size', 'Type']}
								data={[fileMetadata]}
								keys={['lastModified', 'sizeStr', 'mimeType']}
							/>
						</div>
					{/if}

					<div class="flex flex-col h-full w-full overflow-auto text-xs p-4 bg-surface-secondary">
						{#if filePreviewLoading || fileMetadata}
							{#if fileMetadata?.fileKey.endsWith('.png') || fileMetadata?.fileKey.endsWith('.jpg') || fileMetadata?.fileKey.endsWith('.jpeg') || fileMetadata?.fileKey.endsWith('.webp')}
								<div>
									<img
										src={`/api/w/${$workspaceStore}/job_helpers/load_image_preview?file_key=${encodeURIComponent(
											fileMetadata.fileKey
										)}` + (storage ? `&storage=${storage}` : '')}
										alt="S3 preview"
									/>
								</div>
							{:else if fileMetadata?.fileKey.endsWith('.pdf')}
								<div class="w-full h-[950px] border">
									{#await import('$lib/components/display/PdfViewer.svelte')}
										<Loader2 class="animate-spin" />
									{:then Module}
										<Module.default
											source={`/api/w/${$workspaceStore}/job_helpers/load_image_preview?file_key=${encodeURIComponent(
												fileMetadata.fileKey
											)}` + (storage ? `&storage=${storage}` : '')}
										/>
									{/await}
								</div>
							{:else if filePreviewLoading}
								<div class="flex h-6 items-center text-tertiary mb-4">
									<Loader2 size={12} class="animate-spin mr-1" /> File preview loading
								</div>
							{:else if fileMetadata !== undefined && filePreview !== undefined}
								<div class="flex items-center text-tertiary mb-4">
									{#if filePreview.contentType === 'Unknown'}
										Type of file not supported for preview.
									{:else if filePreview.contentType === 'Csv'}
										Previewing a {filePreview.contentType?.toLowerCase()} file. Separator character:
										<div class="inline-flex w-12 ml-2 mr-2">
											<select
												class="h-8"
												bind:value={csvSeparatorChar}
												onchange={(e) =>
													loadFilePreview(
														fileMetadata?.fileKey ?? '',
														fileMetadata?.size,
														fileMetadata?.mimeType
													)}
											>
												<option value=",">,</option>
												<option value=";">;</option>
												<option value="\t">\t</option>
												<option value="|">|</option>
											</select>
										</div>
										Header row:
										<div class="inline-flex item-center w-4 ml-2 mr-2">
											<input
												onfocus={bubble('focus')}
												onclick={bubble('click')}
												disabled={false}
												type="checkbox"
												id="csv-header"
												class="h-5"
												bind:checked={csvHasHeader}
												onchange={stopPropagation((e) =>
													loadFilePreview(
														fileMetadata?.fileKey ?? '',
														fileMetadata?.size,
														fileMetadata?.mimeType
													)
												)}
											/>
										</div>
									{:else}
										Previewing a {filePreview.contentType?.toLowerCase()} file.
									{/if}
								</div>
								<pre class="grow whitespace-no-wrap break-words"
									>{#if !emptyString(filePreview.contentPreview)}{filePreview.contentPreview}{:else if filePreview.contentType !== undefined}Preview impossible.{/if}
							</pre>
							{/if}
						{/if}
					</div>
				</div>
			</div>
		{/if}

		{#snippet actions()}
			<div class="flex gap-1">
				{#if secondaryStorageNames.value?.length}
					<Select
						inputClass="h-10 min-w-44 !placeholder-primary"
						items={[
							{ value: undefined, label: 'Default storage' },
							...secondaryStorageNames.value.map((value) => ({ value }))
						]}
						placeholder="Default storage"
						bind:value={
							() => storage,
							(v) => {
								if (v === storage) return
								storage = v
								reloadContent()
							}
						}
					/>
				{/if}
				{#if !readOnlyMode}
					<Button
						variant="border"
						color="light"
						disabled={workspaceSettingsInitialized === false}
						startIcon={{ icon: FileUp }}
						on:click={() => {
							uploadModalOpen = true
						}}>Upload File</Button
					>
					{#if !fromWorkspaceSettings}
						<Button
							disabled={selectedFileKey === undefined ||
								emptyString(selectedFileKey.s3) ||
								(folderOnly && allFilesByKey[selectedFileKey.s3]?.type !== 'folder')}
							on:click={selectAndClose}>Select</Button
						>
					{/if}
				{/if}
			</div>
		{/snippet}
	</DrawerContent>
</Drawer>

<ConfirmationModal
	open={deletionModalOpen}
	title="Permanently delete file"
	confirmationText="Delete permanently"
	on:canceled={() => {
		deletionModalOpen = false
	}}
	on:confirmed={() => {
		deleteFileFromS3(fileMetadata?.fileKey)
	}}
	keyListen={false}
	loading={fileDeletionInProgress}
>
	<div class="flex flex-col w-full space-y-4">
		<span
			>Are you sure you want to permanently delete {fileMetadata?.fileKey} from the S3 bucket?</span
		>
	</div>
</ConfirmationModal>

<ConfirmationModal
	open={moveModalOpen}
	title="Move file to new location"
	confirmationText="Move"
	on:canceled={() => {
		moveModalOpen = false
	}}
	on:confirmed={() => {
		moveS3File(fileMetadata?.fileKey, moveDestKey)
	}}
	keyListen={false}
	loading={fileMoveInProgress}
>
	<div class="flex flex-col space-y-4">
		<div class="flex items-center justify-between">
			<span class="w-24">New key: </span>
			<input
				type="text"
				placeholder="folder/nested/file.txt"
				bind:value={moveDestKey}
				class="text-2xl"
			/>
		</div>
		<span>Are you sure you want to permanently move {fileMetadata?.fileKey}?</span>
	</div>
</ConfirmationModal>

<FileUploadModal
	open={uploadModalOpen}
	title="Upload file to S3 bucket"
	on:close={async (evt) => {
		uploadModalOpen = false
		if (evt.detail !== undefined && evt.detail !== null) {
			selectedFileKey = { s3: evt.detail, storage }
			await clearAndLoadFiles()
			loadFileMetadataPlusPreviewAsync(evt.detail)
		}
	}}
/>
