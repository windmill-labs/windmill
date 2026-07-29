<script module lang="ts">
	export interface S3TreeNode {
		/** `load_more` is a synthetic row that pages in the rest of a folder. */
		type: 'folder' | 'leaf' | 'load_more'
		full_key: string
		display_name: string
		collapsed: boolean
		parentPath: string | undefined
		nestingLevel: number
		/** Direct children of a shallow-loaded folder, or keys seen under the
		 * folder across flat listing pages. Undefined until known. */
		count: number | undefined
		/** Folders only: whether their direct children are in allFilesByKey. */
		childrenLoaded?: boolean
		/** Folders only: more files remain at that level than have been loaded. */
		hasMore?: boolean
	}
</script>

<script lang="ts">
	import {
		File as FileIcon,
		FolderClosed,
		FolderOpen,
		ChevronDown,
		RotateCw,
		Loader2,
		Download,
		Trash,
		MoveRight
	} from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import {
		CancelablePromise,
		HelpersService,
		type DatasetStorageTestConnectionData,
		type DatasetStorageTestConnectionResponse,
		type DeleteS3FileData,
		type DeleteS3FileResponse,
		type ListStoredFilesData,
		type ListStoredFilesResponse,
		type LoadFileMetadataData,
		type LoadFileMetadataResponse,
		type LoadFilePreviewData,
		type LoadFilePreviewResponse,
		type MoveS3FileData,
		type MoveS3FileResponse
	} from '$lib/gen'
	import { base } from '$lib/base'
	import {
		displayDate,
		displaySize,
		emptyString,
		parseS3Object,
		sendUserToast,
		type S3Object
	} from '$lib/utils'
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'
	import { Alert, Button } from './common'
	import Section from './Section.svelte'
	import { createEventDispatcher, untrack, type Snippet } from 'svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import VirtualList from '@tutorlatin/svelte-tiny-virtual-list'
	import TableSimple from './TableSimple.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import FileUploadModal from './common/fileUpload/FileUploadModal.svelte'
	import S3FilePreview from './S3FilePreview.svelte'
	import { twMerge } from 'tailwind-merge'

	let deletionModalOpen = $state(false)
	let fileDeletionInProgress = $state(false)

	let fileListUnavailable: boolean | undefined = $state(undefined)

	let moveModalOpen = $state(false)
	let moveDestKey: string | undefined = $state(undefined)
	let fileMoveInProgress = $state(false)

	let initialFileKeyInternalCopy: { s3: string; storage?: string }
	interface Props {
		fromWorkspaceSettings?: boolean
		readOnlyMode: boolean
		initialFileKey?: { s3: string; storage?: string } | undefined
		selectedFileKey?: { s3: string; storage?: string } | undefined
		folderOnly?: boolean
		regexFilter?: RegExp | undefined
		hideS3SpecificDetails?: boolean
		rootPath?: string
		/** Workspace to browse S3 storage in — defaults to the nav workspace. */
		workspace?: string | undefined
		workspaceSettingsInitialized?: boolean
		storage?: string | undefined
		/** Browse this object storage resource directly instead of the workspace storage. */
		s3ResourcePath?: string | undefined
		uploadModalOpen?: boolean
		allFilesByKey?: Record<string, S3TreeNode>
		allowDelete?: boolean
		replaceUnauthorizedWarning?: Snippet
		listStoredFilesRequest?: (d: ListStoredFilesData) => CancelablePromise<ListStoredFilesResponse>
		loadFilePreviewRequest?: (d: LoadFilePreviewData) => CancelablePromise<LoadFilePreviewResponse>
		loadFileMetadataRequest?: (
			d: LoadFileMetadataData
		) => CancelablePromise<LoadFileMetadataResponse>
		deleteS3FileRequest?: (d: DeleteS3FileData) => CancelablePromise<DeleteS3FileResponse>
		moveS3FileRequest?: (d: MoveS3FileData) => CancelablePromise<MoveS3FileResponse>
		testConnectionRequest?: (
			d: DatasetStorageTestConnectionData
		) => CancelablePromise<DatasetStorageTestConnectionResponse>
	}

	let {
		fromWorkspaceSettings = false,
		readOnlyMode,
		initialFileKey = $bindable(undefined),
		selectedFileKey = $bindable(undefined),
		folderOnly = false,
		regexFilter = undefined,
		hideS3SpecificDetails = false,
		rootPath: initialRootPath = '',
		workspace = undefined,
		workspaceSettingsInitialized = $bindable(true),
		storage = $bindable(undefined),
		s3ResourcePath = undefined,
		uploadModalOpen = $bindable(false),
		allFilesByKey = $bindable({}),
		allowDelete = false,
		replaceUnauthorizedWarning,
		listStoredFilesRequest = HelpersService.listStoredFiles,
		loadFilePreviewRequest = HelpersService.loadFilePreview,
		loadFileMetadataRequest = HelpersService.loadFileMetadata,
		deleteS3FileRequest = HelpersService.deleteS3File,
		moveS3FileRequest = HelpersService.moveS3File,
		testConnectionRequest = HelpersService.datasetStorageTestConnection
	}: Props = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let rootPath = $state(initialRootPath)
	let rootPathNestingLevel = $derived(1 * (rootPath.split('/').length - 1))

	let csvSeparatorChar: string = $state(',')
	let csvHasHeader: boolean = $state(true)

	let dispatch = createEventDispatcher<{
		close: { s3: string; storage: string | undefined } | undefined
		selectAndClose: { s3: string; storage: string | undefined }
	}>()

	let fileInfoLoading: boolean = $state(true)
	let fileListLoading: boolean = $state(true)
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
	// `filePreviewLoading` was previously templated; now that the visual
	// preview lives in <S3FilePreview/> the picker only uses filePreview to
	// gate the toolbar and refine fileMetadata.mimeType, so the standalone
	// loading flag is gone.
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

	// Set when the server answers a shallow request with a flat listing
	// (advanced LFS permissions apply to this user): keep the flat paginated
	// browsing in that case.
	let shallowUnavailable = $state(false)
	// Prefix search only works on flat listings, so a non-empty filter switches
	// to them too.
	let flatListing = $derived(filter.trim() !== '' || shallowUnavailable)
	let loadingFolderKeys = new SvelteSet<string>()
	/** Resume token per shallow-loaded level, keyed by folder ('' for the root). */
	let nextMarkerByFolder: Record<string, string> = {}
	/** Direct children of each level ('' = no parent), maintained beside
	 * allFilesByKey: per-level operations must not rescan the whole node map,
	 * which is a deep $state proxy, on every expand or load-more click. */
	let childrenByParent: Record<string, string[]> = {}

	function indexChild(parentPath: string | undefined, key: string) {
		;(childrenByParent[parentPath ?? ''] ??= []).push(key)
	}
	function unindexChild(parentPath: string | undefined, key: string) {
		const siblings = childrenByParent[parentPath ?? '']
		const i = siblings?.indexOf(key) ?? -1
		if (i >= 0) {
			siblings.splice(i, 1)
		}
	}
	function childrenOf(parentPath: string | undefined): string[] {
		return childrenByParent[parentPath ?? ''] ?? []
	}
	/** Bumped by every reset; in-flight listings whose generation is stale must
	 * not write into the tree they were started against. */
	let loadGeneration = 0

	function rootParentKey(): string | undefined {
		return rootPath === '' ? undefined : rootPath
	}

	// Sorted after every real child of the folder but before the folder's own
	// siblings, so the row lands at the bottom of the level it pages.
	const LOAD_MORE_SUFFIX = '\uffff'
	function loadMoreKey(folderKey: string | undefined): string {
		return (folderKey ?? '') + LOAD_MORE_SUFFIX
	}

	let timeout: number | undefined = undefined
	let firstLoad = true

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
		if (flatListing) {
			await loadFilesFlat()
			return
		}
		// Browse mode: shallow (delimiter-style) listing of the root level, so a
		// folder with many objects cannot push its siblings out of the page.
		const generation = loadGeneration
		const availableFiles = await listStoredFilesRequest({
			workspace: ws!,
			maxKeys,
			prefix: rootPath === '' ? undefined : rootPath,
			storage,
			s3ResourcePath,
			shallow: true
		})
		if (generation !== loadGeneration) {
			return
		}
		if (availableFiles.restricted_access !== false) {
			fileListUnavailable = true
			loadFileMetadataPlusPreviewAsync(selectedFileKey?.s3)
			return
		}
		fileListUnavailable = false
		if (availableFiles.folders === undefined) {
			// The server answered with a flat page instead: keep flat browsing.
			shallowUnavailable = true
			processFlatPage(availableFiles)
			displayedFileKeys = [...new Set(displayedFileKeys)].sort()
			fileListLoading = false
			fileInfoLoading = false
			return
		}
		processShallowResponse(availableFiles, rootParentKey())
		// The root level paints before the selected file's ancestor chain is
		// opened: the chain costs one request per level and must not hold the
		// whole list hostage.
		displayedFileKeys = [...new Set(displayedFileKeys)].sort()
		fileListLoading = false
		fileInfoLoading = false
		if (selectedFileKey !== undefined && !emptyString(selectedFileKey.s3)) {
			await expandAncestors(selectedFileKey.s3)
		}
	}

	async function loadFilesFlat() {
		const generation = loadGeneration
		let availableFiles = await listStoredFilesRequest({
			workspace: ws!,
			maxKeys: maxKeys, // fixed pages of 1000 files for now
			marker: page == 0 ? undefined : listMarkers[page - 1],
			prefix: rootPath !== '' ? rootPath : filter.trim() !== '' ? filter : undefined,
			storage: storage,
			s3ResourcePath
		})
		// Debounced prefix searches overlap, so a slower earlier one must not
		// land in the tree a later one already rebuilt.
		if (generation !== loadGeneration) {
			return
		}
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
		processFlatPage(availableFiles)
		displayedFileKeys = [...new Set(displayedFileKeys)].sort()
		fileListLoading = false
		fileInfoLoading = false
	}

	function processFlatPage(availableFiles: ListStoredFilesResponse) {
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
				const existing = allFilesByKey[current_path]
				if (existing !== undefined) {
					existing.count = (existing.count ?? 0) + 1
					continue
				}
				allFilesByKey[current_path] = {
					type: i === split_path.length - 1 ? 'leaf' : 'folder',
					full_key: current_path,
					display_name: split_path[i],
					collapsed: true, // folders collapsed by default
					parentPath: parent_path,
					nestingLevel: nestingLevel,
					count: 1,
					// flat pages carry the whole subtree, expanding must not fetch
					childrenLoaded: true
				}
				indexChild(parent_path, current_path)
				if (i == rootPathNestingLevel && current_path.startsWith(rootPath)) {
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

		// un-collapse the folders containing the selected file (if any)
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
	}

	// Returns the number of entries the page contributed to the level.
	function processShallowResponse(
		availableFiles: ListStoredFilesResponse,
		parentKey: string | undefined
	): number {
		let directCount = 0
		for (const folder of availableFiles.folders ?? []) {
			createShallowNode(folder, 'folder', parentKey)
			directCount += 1
		}
		for (const file of availableFiles.windmill_large_files) {
			// A zero-byte folder marker is returned as an object keyed by the
			// folder itself, with the trailing delimiter stripped by the backend's
			// path normalization — so compare against the re-suffixed key, not the
			// raw one. Left in, it renders as a nameless row that sorts above its
			// own parent and survives collapsing.
			if (file.s3 === '' || file.s3 + '/' === parentKey) {
				continue
			}
			if (regexFilter && !regexFilter.test(file.s3)) {
				continue
			}
			createShallowNode(file.s3, 'leaf', parentKey)
			directCount += 1
		}

		const nextMarker = availableFiles.next_marker
		const levelKey = parentKey ?? ''
		if (nextMarker) {
			nextMarkerByFolder[levelKey] = nextMarker
			createLoadMoreNode(parentKey)
		} else {
			delete nextMarkerByFolder[levelKey]
			removeLoadMoreNode(parentKey)
		}

		const parentNode = parentKey !== undefined ? allFilesByKey[parentKey] : undefined
		if (parentNode !== undefined) {
			// additive: paging a folder extends what is already known about it
			parentNode.count = (parentNode.childrenLoaded ? (parentNode.count ?? 0) : 0) + directCount
			parentNode.childrenLoaded = true
			parentNode.hasMore = !!nextMarker
		}
		return directCount
	}

	function createLoadMoreNode(parentKey: string | undefined) {
		const key = loadMoreKey(parentKey)
		if (allFilesByKey[key] !== undefined) {
			return
		}
		const parentNode = parentKey !== undefined ? allFilesByKey[parentKey] : undefined
		allFilesByKey[key] = {
			type: 'load_more',
			full_key: key,
			display_name: '',
			collapsed: true,
			parentPath: parentKey,
			nestingLevel:
				parentNode !== undefined ? parentNode.nestingLevel + 2 : rootPathNestingLevel * 2,
			count: undefined
		}
		indexChild(parentKey, key)
		if (parentKey === rootParentKey()) {
			displayedFileKeys.push(key)
		}
	}

	function removeLoadMoreNode(parentKey: string | undefined) {
		const key = loadMoreKey(parentKey)
		if (allFilesByKey[key] === undefined) {
			return
		}
		delete allFilesByKey[key]
		unindexChild(parentKey, key)
		displayedFileKeys = displayedFileKeys.filter((k) => k !== key)
	}

	// Page in the next batch of files for an already-expanded level.
	async function loadMoreInFolder(parentKey: string | undefined) {
		const levelKey = parentKey ?? ''
		const marker = nextMarkerByFolder[levelKey]
		if (marker === undefined) {
			return
		}
		const generation = loadGeneration
		const loadingKey = loadMoreKey(parentKey)
		loadingFolderKeys.add(loadingKey)
		try {
			await loadShallowFolder(parentKey ?? rootPath, marker)
		} catch (e) {
			sendUserToast(`Could not load more files: ${e}`, true)
			return
		} finally {
			// After a reset the key may already belong to the fresh tree's own
			// request for this folder; a stale settle must not unlock its row.
			if (generation === loadGeneration) {
				loadingFolderKeys.delete(loadingKey)
			}
		}
		// The tree may have been rebuilt, or the folder collapsed, while the page
		// was in flight — either way its children must stay hidden.
		const parentNode = parentKey !== undefined ? allFilesByKey[parentKey] : undefined
		if (generation !== loadGeneration || parentNode?.collapsed === true) {
			return
		}
		revealChildren(parentKey)
	}

	// Make every already-loaded child of a level visible in the flat row list.
	function revealChildren(parentKey: string | undefined) {
		displayedFileKeys.push(...childrenOf(parentKey))
		displayedFileKeys = [...new Set(displayedFileKeys)].sort()
	}

	function createShallowNode(
		full_key: string,
		type: 'folder' | 'leaf',
		parentPath: string | undefined
	) {
		if (allFilesByKey[full_key] !== undefined) {
			return
		}
		displayedCount += 1
		const split_path = full_key.split('/') // folder keys end with '/': last element is ''
		const display_name =
			type === 'folder' ? split_path[split_path.length - 2] : split_path[split_path.length - 1]
		allFilesByKey[full_key] = {
			type,
			full_key,
			display_name,
			collapsed: true, // folders collapsed by default
			parentPath,
			nestingLevel: (split_path.length - (type === 'folder' ? 2 : 1)) * 2,
			count: type === 'leaf' ? 1 : undefined,
			childrenLoaded: type === 'leaf'
		}
		indexChild(parentPath, full_key)
		if (parentPath === rootParentKey()) {
			displayedFileKeys.push(full_key)
		}
	}

	async function loadShallowFolder(folderKey: string, marker?: string) {
		const generation = loadGeneration
		let nextMarker = marker
		// A page can legitimately contribute nothing visible — regexFilter can
		// drop every file in it — so chase a bounded number of further pages
		// rather than render an expand or "load more" click as empty. Bounded:
		// past the cap the load-more row remains and the user decides.
		for (let pagesFetched = 0; pagesFetched < 5; pagesFetched++) {
			const availableFiles = await listStoredFilesRequest({
				workspace: ws!,
				maxKeys,
				prefix: folderKey === '' ? undefined : folderKey,
				marker: nextMarker,
				storage,
				s3ResourcePath,
				shallow: true
			})
			if (
				generation !== loadGeneration ||
				availableFiles.restricted_access !== false ||
				availableFiles.folders === undefined
			) {
				return
			}
			const added = processShallowResponse(
				availableFiles,
				folderKey === '' ? undefined : folderKey
			)
			nextMarker = availableFiles.next_marker ?? undefined
			if (added > 0 || nextMarker === undefined) {
				return
			}
		}
	}

	// Open the ancestor folders of fileKey below rootPath so its node exists
	// and is visible. Every level is derivable from the key upfront and the
	// listings are independent, so they load concurrently — a deep chain must
	// not cost one round-trip per level in series.
	async function expandAncestors(fileKey: string) {
		const generation = loadGeneration
		const split_path = fileKey.split('/')
		const chain: { prefix: string; parent: string | undefined; synthesized: boolean }[] = []
		let prefix = ''
		for (let i = 0; i < split_path.length - 1; i++) {
			const parent = prefix === '' ? rootParentKey() : prefix
			prefix += split_path[i] + '/'
			if (rootPath.startsWith(prefix)) {
				continue // at or above the browsed root
			}
			chain.push({ prefix, parent, synthesized: false })
		}
		for (const link of chain) {
			// Subfolders surface as their level is paged, so an ancestor sitting
			// past the first page of a wide level has no node yet — the path is
			// enough to place one, and the loads below prove whether it exists.
			if (allFilesByKey[link.prefix] === undefined) {
				createShallowNode(link.prefix, 'folder', link.parent)
				link.synthesized = true
			}
		}
		// Registering in loadingFolderKeys locks each folder's row while its
		// page is in flight, so a click on an already-visible ancestor cannot
		// start the same first-page request a second time.
		const toLoad = chain.filter(
			(l) => allFilesByKey[l.prefix]?.childrenLoaded !== true && !loadingFolderKeys.has(l.prefix)
		)
		for (const link of toLoad) {
			loadingFolderKeys.add(link.prefix)
		}
		try {
			await Promise.all(toLoad.map((l) => loadShallowFolder(l.prefix)))
		} finally {
			// After a reset the keys may already belong to the fresh tree's own
			// requests; a stale settle must not unlock its rows.
			if (generation === loadGeneration) {
				for (const link of toLoad) {
					loadingFolderKeys.delete(link.prefix)
				}
			}
		}
		// A reload during the fetches left the chain pointing at discarded
		// nodes; expanding them now would show empty open folders.
		if (generation !== loadGeneration) {
			return
		}
		for (const [i, link] of chain.entries()) {
			const folder = allFilesByKey[link.prefix]
			const stale =
				folder === undefined ||
				folder.type !== 'folder' ||
				(link.synthesized && (folder.count ?? 0) === 0)
			if (stale) {
				// Nothing under this level: the path is stale (a delete may have
				// just emptied it). Every synthesized node from here down is a
				// phantom of the same dead path — drop them all, including their
				// contribution to the loaded-items footer.
				for (const tail of chain.slice(i)) {
					if (tail.synthesized && allFilesByKey[tail.prefix] !== undefined) {
						delete allFilesByKey[tail.prefix]
						unindexChild(tail.parent, tail.prefix)
						displayedFileKeys = displayedFileKeys.filter((k) => k !== tail.prefix)
						displayedCount -= 1
					}
				}
				break
			}
			folder.collapsed = false
			// The whole chain is being opened, so the folder belongs on screen
			// itself — a synthesized one below the root level is not covered by
			// the reveal loops, and its children would render with no row above.
			displayedFileKeys.push(link.prefix)
			displayedFileKeys.push(...childrenOf(link.prefix))
		}
		displayedFileKeys = [...new Set(displayedFileKeys)].sort()
	}

	async function loadFileMetadataPlusPreviewAsync(fileKey: string | undefined) {
		if (fileKey === undefined || emptyString(fileKey)) {
			fileInfoLoading = false
			return
		}
		fileInfoLoading = true
		let fileMetadataRaw = await loadFileMetadataRequest({
			workspace: ws!,
			fileKey: fileKey,
			storage: storage,
			s3ResourcePath
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
		let filePreviewRaw = await loadFilePreviewRequest({
			workspace: ws!,
			fileKey: fileKey,
			fileSizeInBytes: fileSizeInBytes,
			fileMimeType: fileMimeType,
			csvSeparator: csvSeparatorChar,
			csvHasHeader: csvHasHeader,
			readBytesFrom: 0,
			readBytesLength: 128 * 1024, // For now static limit of 128Kb per file,
			storage: storage,
			s3ResourcePath
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
		fileInfoLoading = false
	}

	async function deleteFileFromS3(fileKey: string | undefined) {
		fileDeletionInProgress = true
		if (fileKey === undefined) {
			return
		}
		try {
			await deleteS3FileRequest({
				workspace: ws!,
				fileKey: fileKey,
				storage: storage,
				s3ResourcePath
			})
		} finally {
			fileDeletionInProgress = false
			deletionModalOpen = false
		}
		sendUserToast(`${fileKey} deleted from S3 bucket`)
		selectedFileKey = { s3: '', storage }
		const currentPage = page
		await clearAndLoadFiles()
		if (!flatListing) {
			// re-open the folder the deleted file was in (stops early if the
			// deletion emptied it out of existence)
			await expandAncestors(fileKey)
		} else {
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
		}
		displayedFileKeys = [...new Set(displayedFileKeys)].sort()
	}

	async function clearAndLoadFiles({ keepFilter }: { keepFilter?: boolean } = {}) {
		loadGeneration += 1
		displayedFileKeys = []
		allFilesByKey = {}
		count = 0
		displayedCount = 0
		page = 0
		listMarkers = []
		nextMarkerByFolder = {}
		childrenByParent = {}
		loadingFolderKeys.clear()
		// re-detect on every reload: switching storage can change whether the
		// server serves shallow listings
		shallowUnavailable = false
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
			await moveS3FileRequest({
				workspace: ws!,
				srcFileKey: srcFileKey,
				destFileKey: destFileKey!,
				storage: storage,
				s3ResourcePath
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

	export async function open(_preSelectedFileKey: S3Object | undefined = undefined) {
		const preSelectedFileKey = _preSelectedFileKey && parseS3Object(_preSelectedFileKey)
		storage = preSelectedFileKey?.storage
		if (preSelectedFileKey !== undefined && preSelectedFileKey.s3.endsWith('/')) {
			rootPath = preSelectedFileKey.s3
			filter = ''
			selectedFileKey = undefined
		} else if (preSelectedFileKey !== undefined) {
			rootPath = ''
			initialFileKey = { ...preSelectedFileKey }
			selectedFileKey = { ...preSelectedFileKey }
		} else {
			rootPath = ''
		}
		reloadContent()
	}

	export async function close() {
		return selectedFileKey?.s3
			? {
					s3: selectedFileKey.s3,
					storage: storage
				}
			: undefined
	}

	export async function reloadContent() {
		if (initialFileKey !== undefined) {
			initialFileKeyInternalCopy = { ...initialFileKey }
		}
		fileListLoading = true
		try {
			await testConnectionRequest({
				workspace: ws!,
				storage: storage,
				s3ResourcePath
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
			} else if (allFilesByKey[selectedFileKey.s3].type !== 'folder') {
				loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
			}
		}
	}

	export async function selectAndClose() {
		if (selectedFileKey?.s3) {
			dispatch('selectAndClose', { s3: selectedFileKey.s3, storage })
		}
	}

	export async function exit() {
		if (initialFileKeyInternalCopy !== undefined) {
			selectedFileKey = { ...initialFileKeyInternalCopy }
		}
	}

	async function selectItem(index: number, toggleCollapsed: boolean = true) {
		let item_key = displayedFileKeys[index]
		let item = allFilesByKey[item_key]
		if (item.type === 'load_more') {
			if (!loadingFolderKeys.has(item_key)) {
				await loadMoreInFolder(item.parentPath)
			}
		} else if (item.type === 'folder') {
			if (folderOnly) {
				selectedFileKey = {
					s3: item_key,
					storage
				}
			}
			if (loadingFolderKeys.has(item_key)) {
				return
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
				if (item.childrenLoaded === false) {
					const generation = loadGeneration
					loadingFolderKeys.add(item_key)
					try {
						await loadShallowFolder(item_key)
					} catch (e) {
						item.collapsed = true
						sendUserToast(`Could not load folder content: ${e}`, true)
						return
					} finally {
						// After a reset the key may already belong to the fresh
						// tree's own request; a stale settle must not unlock it.
						if (generation === loadGeneration) {
							loadingFolderKeys.delete(item_key)
						}
					}
					// A reload during the fetch replaced the node this click
					// toggled, so revealing children now would show rows under a
					// folder the fresh tree still draws as closed.
					if (generation !== loadGeneration) {
						return
					}
				}
				// Re-add the currently hidden element to displayed_file_keys
				for (const file_key of childrenOf(item_key)) {
					const file_info = allFilesByKey[file_key]
					if (file_info === undefined) {
						continue
					}
					displayedFileKeys.push(file_key)
					if (file_info.type === 'folder' && !file_info.collapsed) {
						selectItem(displayedFileKeys.length - 1, false)
					}
				}
			}
			displayedFileKeys = [...new Set(displayedFileKeys)].sort()
		} else {
			selectedFileKey = {
				s3: item_key,
				storage
			}
			loadFileMetadataPlusPreviewAsync(selectedFileKey.s3)
		}
	}
	$effect.pre(() => {
		filter != undefined && untrack(() => onFilterChange())
	})
</script>

{#if workspaceSettingsInitialized === false}
	{#if fromWorkspaceSettings}
		<Alert type="error" title="Connection to remote S3 bucket unsuccessful">
			<div class="flex flex-row gap-x-1 w-full items-center">
				<p class="text-clip grow min-w-0"> Double check the S3 resource fields and try again. </p>
			</div>
		</Alert>
	{:else if s3ResourcePath}
		<Alert type="error" title="Could not connect to the object storage of {s3ResourcePath}">
			<div class="flex flex-row gap-x-1 w-full items-center">
				<p class="text-clip grow min-w-0">
					Double check the resource fields and that its object storage is reachable, then try again.
				</p>
				<Button variant="default" on:click={reloadContent} startIcon={{ icon: RotateCw }} />
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
				<Button variant="default" on:click={reloadContent} startIcon={{ icon: RotateCw }} />
			</div>
		</Alert>
	{/if}
{:else}
	{#if fileListUnavailable == true}
		{#if replaceUnauthorizedWarning}
			{@render replaceUnauthorizedWarning()}
		{:else}
			<div class="mb-2">
				<Alert type="info" title="Access to S3 bucket restricted">
					<p>
						You don't have access to the S3 bucket resource and your administrator has restricted
						the access to it. You are not authorized to browse the bucket content. If you think this
						is incorrect, please contact your workspace administrator.
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
	{/if}
	<div class="flex flex-row border rounded-md h-full min-h-0 overflow-hidden">
		{#if !fileListUnavailable}
			<div class="min-w-[30%] border-r flex flex-col min-h-0">
				{#if !rootPath}
					<div class="w-full p-1 border-b">
						<input type="text" placeholder="Folder prefix" bind:value={filter} class="text-xl" />
					</div>
				{/if}
				{#if displayedFileKeys.length === 0}
					{#if fileListLoading}
						<div class="grow min-h-0 flex justify-center items-center">
							<div class="flex text-secondary text-xs items-center">
								<Loader2 size={12} class="animate-spin mr-1" /> Loading content
							</div>
						</div>
					{:else}
						<div class="p-4 text-primary text-xs text-center italic">
							{#if filter.trim() !== ''}
								No files under "{filter.trim()}" — the prefix has to match whole folder names
							{:else}
								No files in the workspace S3 bucket at that prefix
							{/if}
						</div>
					{/if}
				{:else}
					<div class="grow min-h-0" bind:clientHeight={listDivHeight}>
						<VirtualList
							width="100%"
							height={listDivHeight}
							itemCount={displayedFileKeys.length}
							itemSize={42}
						>
							{#snippet header()}{/snippet}
							{#snippet footer()}{/snippet}
							{#snippet item({ index, style })}
								{@const file_info = allFilesByKey[displayedFileKeys[index]]}

								<div
									{style}
									class={twMerge(
										'hover:bg-surface-hover border-b',
										index === displayedFileKeys.length - 1 && 'border-b-0'
									)}
								>
									{#if file_info}
										{@const nestingLevel = file_info.nestingLevel - 2 * rootPathNestingLevel}
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											onclick={() => selectItem(index)}
											class={twMerge(
												'flex flex-row h-full font-semibold text-xs items-center justify-start',
												selectedFileKey !== undefined && selectedFileKey.s3 === file_info.full_key
													? 'bg-surface-hover'
													: ''
											)}
										>
											<div
												class={`flex flex-row w-full gap-2 h-full items-center`}
												style={`margin-left: ${(2 + nestingLevel) * 0.25}rem;`}
											>
												{#if file_info.type === 'load_more'}
													{#if loadingFolderKeys.has(file_info.full_key)}<Loader2
															size={16}
															class="animate-spin"
														/>{:else}<ChevronDown size={16} />{/if}
													<div class="truncate text-ellipsis w-56 text-secondary font-normal">
														{loadingFolderKeys.has(file_info.full_key)
															? 'Loading…'
															: 'Load more in this folder'}
													</div>
												{:else if file_info.type === 'folder'}
													{#if loadingFolderKeys.has(file_info.full_key)}<Loader2
															size={16}
															class="animate-spin"
														/>{:else if file_info.collapsed}<FolderClosed
															size={16}
														/>{:else}<FolderOpen size={16} />{/if}
													<div class="truncate text-ellipsis w-56">
														{file_info.display_name}
														{#if file_info.count !== undefined}
															({file_info.count}{file_info.hasMore ||
															(flatListing &&
																count % 1000 === 0 &&
																lastKeyFolders[file_info.nestingLevel / 2] ===
																	file_info.display_name)
																? '+'
																: ''} item{file_info.count === 1 ? '' : 's'})
														{/if}
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
						{:else if !flatListing}
							<div>
								{displayedCount} item{displayedCount === 1 ? '' : 's'} loaded
							</div>
						{:else}
							<div>
								{displayedCount}{count % maxKeys === 0 ? '+' : ''}
								{displayedCount !== count ? 'filtered ' : ''}items (including inside folders)
							</div>

							{#if count % maxKeys === 0}
								<Button
									variant="default"
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
		<div class="flex flex-col h-full w-full min-h-0 overflow-hidden">
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
				<div class="px-3 py-2 flex flex-col gap-2">
					<div class="flex flex-row items-center justify-between gap-2">
						<h2 class="text-emphasis text-sm font-semibold break-all min-w-0">
							{((p) => (p.startsWith(rootPath) ? p.slice(rootPath.length) : p))(
								fileMetadata.fileKey
							)}
						</h2>
						{#if filePreview !== undefined && (!hideS3SpecificDetails || !readOnlyMode || allowDelete)}
							<div class="flex gap-2 shrink-0">
								{#if !hideS3SpecificDetails}
									{@const downloadApiPath = `/w/${ws}/job_helpers/download_s3_file?file_key=${encodeURIComponent(fileMetadata?.fileKey ?? '')}${storage ? `&storage=${storage}` : ''}${s3ResourcePath ? `&s3_resource_path=${encodeURIComponent(s3ResourcePath)}` : ''}`}
									{@const downloadName =
										fileMetadata?.fileKey.split('/').pop() ?? 'unnamed_download.file'}
									{#if shouldDownloadViaClient()}
										<Button
											title="Download file from S3"
											variant="default"
											on:click={() => downloadViaClient(downloadApiPath, downloadName)}
											startIcon={{ icon: Download }}
											iconOnly={true}
										/>
									{:else}
										<Button
											title="Download file from S3"
											variant="default"
											href={`${base}/api${downloadApiPath}`}
											download={downloadName}
											startIcon={{ icon: Download }}
											iconOnly={true}
										/>
									{/if}
								{/if}
								{#if !readOnlyMode}
									<Button
										title="Move file"
										variant="default"
										on:click={() => {
											moveDestKey = fileMetadata?.fileKey ?? ''
											moveModalOpen = true
										}}
										startIcon={{ icon: MoveRight }}
										iconOnly={true}
									/>
								{/if}
								{#if !readOnlyMode || allowDelete}
									<Button
										title="Delete file"
										variant="default"
										on:click={() => {
											deletionModalOpen = true
										}}
										startIcon={{ icon: Trash }}
										iconOnly={true}
									/>
								{/if}
							</div>
						{/if}
					</div>
					{#if !hideS3SpecificDetails}
						<TableSimple
							headers={['Last modified', 'Size', 'Type']}
							data={[fileMetadata]}
							keys={['lastModified', 'sizeStr', 'mimeType']}
						/>
					{/if}
				</div>
			{/if}

			<!-- Visual preview extracted to a standalone S3FilePreview component
			     so the asset detail pane (and other surfaces) can render the
			     same image/PDF/CSV/text views without dragging in the rest
			     of the picker. The picker keeps its own metadata loading
			     above for the download/move toolbar; S3FilePreview does an
			     independent load — fine on this non-hot path, and avoids
			     plumbing pre-loaded state through component boundaries. -->
			<S3FilePreview
				fileKey={fileMetadata?.fileKey}
				{storage}
				{s3ResourcePath}
				workspace={ws}
				{loadFilePreviewRequest}
				{loadFileMetadataRequest}
				class="h-full"
			/>
		</div>
	</div>
{/if}

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
