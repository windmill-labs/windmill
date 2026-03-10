<script lang="ts">
	import SimpleEditor from './SimpleEditor.svelte'
	import FileExplorer from './FileExplorer.svelte'

	interface Props {
		args: Record<string, any>
	}

	let { args = $bindable({}) }: Props = $props()

	// Internal files map uses /-prefixed keys (matching tree node paths).
	// Compute initial files + selection together to avoid referencing $state outside reactive context.
	const initialFiles = Object.fromEntries(
		Object.entries(args ?? {}).map(([k, v]) => ['/' + k, String(v ?? '')])
	)
	const initialFile = Object.keys(initialFiles).find((k) => !k.endsWith('/'))

	let files: Record<string, string> = $state(initialFiles)
	let selectedPath: string | undefined = $state(initialFile ?? '/')
	let editContent: string = $state(initialFile ? (initialFiles[initialFile] ?? '') : '')

	// The selected file path (/-prefixed, not a folder)
	const selectedFileKey: string | undefined = $derived.by(() => {
		if (selectedPath != null && !selectedPath.endsWith('/')) {
			return selectedPath
		}
		return undefined
	})

	// Display key without leading /
	const selectedDisplayKey: string | undefined = $derived(
		selectedFileKey?.replace(/^\//, '')
	)

	function flushEditContent() {
		if (selectedFileKey != null && selectedFileKey in files && files[selectedFileKey] !== editContent) {
			files = { ...files, [selectedFileKey]: editContent }
		}
	}

	function handleSelectPath(path: string) {
		flushEditContent()
		selectedPath = path
		if (!path.endsWith('/') && path !== '') {
			editContent = files[path] ?? ''
		}
	}

	// Track the last args we wrote so we can detect external changes.
	let lastWrittenArgs: Record<string, any> = $state(args ?? {})

	// Sync files → args, overlaying current editContent for the active file.
	// This avoids spreading a new files object on every keystroke.
	$effect(() => {
		const currentKey = selectedFileKey
		const currentContent = editContent
		const newArgs: Record<string, any> = {}
		for (const [key, value] of Object.entries(files)) {
			if (!key.endsWith('/')) {
				const argKey = key.replace(/^\//, '')
				newArgs[argKey] = key === currentKey ? currentContent : value
			}
		}
		lastWrittenArgs = newArgs
		args = newArgs
	})

	// Sync args → files when args changes externally (e.g. from AI generation).
	$effect(() => {
		const currentArgs = args
		if (currentArgs === lastWrittenArgs) return
		// Check if the args object is actually different
		const currentKeys = Object.keys(currentArgs ?? {}).sort().join('\0')
		const lastKeys = Object.keys(lastWrittenArgs ?? {}).sort().join('\0')
		if (currentKeys === lastKeys) {
			const allSame = Object.entries(currentArgs ?? {}).every(
				([k, v]) => lastWrittenArgs[k] === v
			)
			if (allSame) return
		}
		const newFiles = Object.fromEntries(
			Object.entries(currentArgs ?? {}).map(([k, v]) => ['/' + k, String(v ?? '')])
		)
		files = newFiles
		lastWrittenArgs = currentArgs
		const firstFile = Object.keys(newFiles).find((k) => !k.endsWith('/'))
		selectedPath = firstFile ?? '/'
		editContent = firstFile ? (newFiles[firstFile] ?? '') : ''
	})

	function inferLang(filePath: string): string {
		const ext = filePath.split('.').pop()?.toLowerCase()
		if (!ext) return 'plaintext'
		const langMap: Record<string, string> = {
			json: 'json',
			yaml: 'yaml',
			yml: 'yaml',
			toml: 'toml',
			ini: 'ini',
			xml: 'xml',
			html: 'html',
			css: 'css',
			js: 'javascript',
			ts: 'typescript',
			py: 'python',
			sh: 'shell',
			bash: 'shell',
			sql: 'sql',
			md: 'markdown',
			cfg: 'ini',
			conf: 'ini',
			j2: 'jinja',
			jinja: 'jinja'
		}
		return langMap[ext] ?? 'plaintext'
	}
</script>

<div class="flex border rounded-md overflow-hidden h-[60vh]">
	<div class="w-56 shrink-0 border-r flex flex-col bg-surface-secondary overflow-y-auto">
		<FileExplorer
			bind:files
			{selectedPath}
			onSelectPath={handleSelectPath}
			showRoot
		/>
	</div>
	<div class="flex-1 min-w-0 overflow-y-auto">
		{#if selectedDisplayKey != null}
			<div class="px-2 border-b text-xs text-secondary bg-surface-secondary sticky top-0 z-10 flex items-center h-[36.5px]">
				{selectedDisplayKey}
			</div>
			{#key selectedFileKey}
				<SimpleEditor
					autoHeight
					lang={inferLang(selectedDisplayKey)}
					bind:code={editContent}
					fixedOverflowWidgets={false}
				/>
			{/key}
		{:else}
			<div class="flex items-center justify-center h-full text-xs text-secondary">
				Select a file or add a new one
			</div>
		{/if}
	</div>
</div>
