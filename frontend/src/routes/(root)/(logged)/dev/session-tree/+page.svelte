<script lang="ts">
	// DEV-ONLY design exploration: how to visually distinguish a *workspace* from a
	// *session* in the rail tree, and make the hierarchy (forks) clear. Not linked
	// anywhere; open at /dev/session-tree. Pure mock data, no real session state.
	// Only reachable on dev builds.
	import WorkspaceIcon from '$lib/components/workspace/WorkspaceIcon.svelte'
	import {
		MessageSquare,
		PencilLine,
		ChevronRight,
		CornerDownRight,
		Folder,
		FileText,
		Building,
		GitFork
	} from 'lucide-svelte'

	type Sess = {
		id: string
		name: string
		unread?: number
		draft?: boolean
		archived?: boolean
	}
	type Node = {
		id: string
		name: string
		color: string
		depth: number
		isFork?: boolean
		parentName?: string
		sessions: Sess[]
	}

	// One rich dataset covering: empty workspace, selected session, unread, draft,
	// archived, and fork depth 1 + 2.
	const data: Node[] = [
		{ id: 'admins', name: 'Admins', color: '#6b7280', depth: 0, sessions: [] },
		{
			id: 'ai-meta',
			name: 'AI Meta E2E',
			color: '#3b82f6',
			depth: 0,
			sessions: [
				{ id: 's-refactor', name: 'Refactor the parser' },
				{ id: 's-flaky', name: 'Fix flaky test', unread: 3 },
				{ id: 's-draft', name: 'New idea', draft: true }
			]
		},
		{
			id: 'rawapp',
			name: 'raw-app-parser',
			color: '#10b981',
			depth: 0,
			sessions: [{ id: 's-crud', name: 'Build CRUD app' }]
		},
		{
			id: 'rawapp-fork',
			name: 'organized-fork',
			color: '#f59e0b',
			depth: 1,
			isFork: true,
			parentName: 'raw-app-parser',
			sessions: [{ id: 's-org', name: 'Organize files', archived: true }]
		},
		{
			id: 'rawapp-fork2',
			name: 'deep-fork',
			color: '#ec4899',
			depth: 2,
			isFork: true,
			parentName: 'organized-fork',
			sessions: [{ id: 's-deep', name: 'Deep work', unread: 1 }]
		}
	]

	const states = [
		{ label: 'A session selected', sel: 's-refactor', browsed: '' },
		{ label: 'Browsing a workspace (no chat)', sel: '', browsed: 'rawapp' }
	]

	const solutions = [
		{ key: 'A', title: 'A · Avatar vs dot + indent', render: solutionA },
		{ key: 'B', title: 'B · Folders & files + workspace-tree lines', render: solutionB },
		{ key: 'B2', title: 'B2 · Workspace avatars (smaller) · no session icon', render: solutionB2 },
		{ key: 'C', title: 'C · Workspace as muted header', render: solutionC },
		{ key: 'D', title: 'D · Workspace cards', render: solutionD }
	]
</script>

{#snippet unread(n: number)}
	<span
		class="ml-auto shrink-0 inline-flex items-center justify-center rounded-full bg-surface-accent-primary text-white font-medium leading-none min-w-4 h-4 px-1 text-[10px]"
	>
		{n > 9 ? '9+' : n}
	</span>
{/snippet}

<!-- ───────────────── Solution A: avatar (workspace) vs dot (session) + indent ──── -->
{#snippet solutionA(sel: string, browsed: string)}
	<div class="flex flex-col gap-0.5 p-2">
		{#each data as ws (ws.id)}
			<button
				type="button"
				style:padding-left={`${8 + ws.depth * 14}px`}
				class="flex items-center gap-2 w-full text-left pr-2 py-1 rounded {browsed === ws.id
					? 'bg-surface-hover'
					: 'hover:bg-surface-hover'}"
			>
				<WorkspaceIcon workspaceColor={ws.color} isForked={ws.isFork} size={12} />
				<span
					class="text-xs font-medium truncate {browsed === ws.id
						? 'text-emphasis'
						: 'text-primary'}">{ws.name}</span
				>
			</button>
			{#each ws.sessions as s (s.id)}
				<button
					type="button"
					style:padding-left={`${8 + ws.depth * 14 + 18}px`}
					class="flex items-center gap-2 w-full text-left pr-2 py-1 rounded {sel === s.id
						? 'bg-surface-hover'
						: 'hover:bg-surface-hover'} {s.archived ? 'opacity-50' : ''}"
				>
					<MessageSquare size={12} class="shrink-0 text-tertiary" />
					<span
						class="text-xs truncate {sel === s.id
							? 'text-emphasis font-semibold'
							: 'text-secondary'}">{s.name}</span
					>
					{#if s.draft}<PencilLine size={11} class="ml-auto shrink-0 text-tertiary" />{/if}
					{#if s.unread}{@render unread(s.unread)}{/if}
				</button>
			{/each}
		{/each}
	</div>
{/snippet}

<!-- ───────────────── Solution B: folders (workspaces) & files (sessions) ──────────
     Workspaces are folders, sessions are files. The connecting lines render the
     workspace tree (fork nesting / containment) — not the session elements. Small
     folder/file icons. -->
{#snippet vline()}
	<span class="relative w-3.5 shrink-0 self-stretch">
		<span class="absolute inset-y-0 left-1/2 w-px bg-gray-200 dark:bg-gray-700"></span>
	</span>
{/snippet}
{#snippet solutionB(sel: string, browsed: string)}
	<div class="flex flex-col py-1">
		{#each data as ws (ws.id)}
			<button
				type="button"
				class="flex items-stretch w-full text-left pr-2 {browsed === ws.id
					? 'bg-surface-hover'
					: 'hover:bg-surface-hover'}"
			>
				{#each Array(ws.depth) as _}{@render vline()}{/each}
				<span class="flex items-center gap-1.5 py-1 pl-1 min-w-0">
					<Folder size={13} class="shrink-0" style="color: {ws.color}" />
					<span
						class="text-xs truncate {browsed === ws.id
							? 'text-emphasis font-semibold'
							: 'text-primary font-medium'}">{ws.name}</span
					>
				</span>
			</button>
			{#each ws.sessions as s (s.id)}
				<button
					type="button"
					class="flex items-stretch w-full text-left pr-2 {sel === s.id
						? 'bg-surface-hover'
						: 'hover:bg-surface-hover'} {s.archived ? 'opacity-50' : ''}"
				>
					{#each Array(ws.depth + 1) as _}{@render vline()}{/each}
					<span class="flex items-center gap-1.5 py-1 pl-1 min-w-0 flex-1">
						<FileText size={12} class="shrink-0 text-tertiary" />
						<span
							class="text-xs truncate {sel === s.id
								? 'text-emphasis font-semibold'
								: 'text-secondary'}">{s.name}</span
						>
						{#if s.draft}<PencilLine size={11} class="ml-auto shrink-0 text-tertiary" />{/if}
						{#if s.unread}{@render unread(s.unread)}{/if}
					</span>
				</button>
			{/each}
		{/each}
	</div>
{/snippet}

<!-- ───────────────── Solution B2: classic workspace/fork avatars (smaller),
     no icon on sessions, same workspace-tree guide lines ──────────────────────── -->
{#snippet solutionB2(sel: string, browsed: string)}
	<div class="flex flex-col py-1">
		{#each data as ws (ws.id)}
			<button
				type="button"
				class="flex items-stretch w-full text-left pr-2 {browsed === ws.id
					? 'bg-surface-hover'
					: 'hover:bg-surface-hover'}"
			>
				{#each Array(ws.depth) as _}{@render vline()}{/each}
				<span class="flex items-center gap-1.5 py-1 pl-1 min-w-0">
					{#if ws.isFork}
						<GitFork size={14} class="shrink-0" style="color: {ws.color}" />
					{:else}
						<Building size={14} class="shrink-0" style="color: {ws.color}" />
					{/if}
					<span
						class="text-xs truncate {browsed === ws.id
							? 'text-emphasis font-medium'
							: 'text-primary'}">{ws.name}</span
					>
				</span>
			</button>
			{#each ws.sessions as s (s.id)}
				<button
					type="button"
					class="flex items-stretch w-full text-left pr-2 {sel === s.id
						? 'bg-surface-hover'
						: 'hover:bg-surface-hover'} {s.archived ? 'opacity-50' : ''}"
				>
					{#each Array(ws.depth + 1) as _}{@render vline()}{/each}
					<span class="flex items-center gap-2 py-1 pl-1 min-w-0 flex-1">
						<span
							class="text-xs truncate {sel === s.id
								? 'text-emphasis font-medium'
								: 'text-secondary'}">{s.name}</span
						>
						{#if s.draft}<PencilLine size={11} class="ml-auto shrink-0 text-tertiary" />{/if}
						{#if s.unread}{@render unread(s.unread)}{/if}
					</span>
				</button>
			{/each}
		{/each}
	</div>
{/snippet}

<!-- ───────────────── Solution C: workspace as muted uppercase header ──────────── -->
{#snippet solutionC(sel: string, browsed: string)}
	<div class="flex flex-col p-2">
		{#each data as ws, wi (ws.id)}
			<button
				type="button"
				style:padding-left={`${8 + ws.depth * 14}px`}
				class="flex items-center gap-1.5 w-full text-left pr-2 py-1 rounded {wi > 0
					? 'mt-2'
					: ''} {browsed === ws.id ? 'bg-surface-hover' : 'hover:bg-surface-hover'}"
			>
				<WorkspaceIcon workspaceColor={ws.color} isForked={ws.isFork} size={10} />
				<span
					class="text-[0.6rem] uppercase tracking-wide font-semibold truncate {browsed === ws.id
						? 'text-emphasis'
						: 'text-tertiary'}">{ws.name}</span
				>
				{#if ws.isFork}<span class="text-[0.55rem] text-tertiary">· fork</span>{/if}
			</button>
			{#each ws.sessions as s (s.id)}
				<button
					type="button"
					style:padding-left={`${8 + ws.depth * 14 + 6}px`}
					class="flex items-center gap-2 w-full text-left pr-2 py-1 rounded {sel === s.id
						? 'bg-surface-hover'
						: 'hover:bg-surface-hover'} {s.archived ? 'opacity-50' : ''}"
				>
					<MessageSquare size={13} class="shrink-0 text-blue-500" />
					<span
						class="text-xs truncate {sel === s.id ? 'text-emphasis font-semibold' : 'text-primary'}"
						>{s.name}</span
					>
					{#if s.draft}<PencilLine size={11} class="ml-auto shrink-0 text-tertiary" />{/if}
					{#if s.unread}{@render unread(s.unread)}{/if}
				</button>
			{/each}
		{/each}
	</div>
{/snippet}

<!-- ───────────────── Solution D: workspace cards ──────────────────────────────── -->
{#snippet solutionD(sel: string, browsed: string)}
	<div class="flex flex-col gap-1.5 p-2">
		{#each data as ws (ws.id)}
			<div style:margin-left={`${ws.depth * 12}px`}>
				<button
					type="button"
					class="flex items-center gap-2 w-full text-left px-2 py-1.5 rounded-t-md {browsed ===
					ws.id
						? 'bg-surface-accent-selected'
						: 'bg-surface-secondary hover:bg-surface-hover'} {ws.sessions.length
						? ''
						: 'rounded-b-md'}"
				>
					<WorkspaceIcon workspaceColor={ws.color} isForked={ws.isFork} size={12} />
					<span class="text-xs font-semibold truncate text-primary">{ws.name}</span>
					{#if ws.isFork}<CornerDownRight size={11} class="ml-auto shrink-0 text-tertiary" />{/if}
				</button>
				{#if ws.sessions.length}
					<div
						class="border border-t-0 border-light rounded-b-md divide-y divide-light overflow-hidden"
					>
						{#each ws.sessions as s (s.id)}
							<button
								type="button"
								class="flex items-center gap-2 w-full text-left px-2 py-1 {sel === s.id
									? 'bg-surface-hover'
									: 'hover:bg-surface-hover'} {s.archived ? 'opacity-50' : ''}"
							>
								<MessageSquare size={12} class="shrink-0 text-tertiary" />
								<span
									class="text-xs truncate {sel === s.id
										? 'text-emphasis font-semibold'
										: 'text-secondary'}">{s.name}</span
								>
								{#if s.draft}<PencilLine size={11} class="ml-auto shrink-0 text-tertiary" />{/if}
								{#if s.unread}{@render unread(s.unread)}{/if}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		{/each}
	</div>
{/snippet}

{#if import.meta.env.DEV}
	<div class="h-full overflow-auto p-6 bg-surface">
		<div class="max-w-[1400px] mx-auto flex flex-col gap-2">
			<h1 class="text-xl font-semibold text-primary flex items-center gap-2">
				<Folder size={18} /> Session vs Workspace tree — design exploration
			</h1>
			<p class="text-sm text-secondary">
				Dev-only mock. Each solution distinguishes <strong>workspaces</strong> (clickable, open a
				preview) from <strong>sessions</strong> (open the chat), and shows the fork hierarchy. Two states
				per solution: a session selected, and a workspace being browsed.
			</p>

			<div class="mt-4 flex flex-col gap-8">
				{#each solutions as sol (sol.key)}
					<section class="flex flex-col gap-2">
						<h2 class="text-sm font-semibold text-emphasis">{sol.title}</h2>
						<div class="flex flex-row flex-wrap gap-4">
							{#each states as st (st.label)}
								<div class="flex flex-col gap-1">
									<span class="text-xs text-tertiary flex items-center gap-1">
										<ChevronRight size={12} />{st.label}
									</span>
									<div
										class="w-64 rounded-md border border-light bg-surface-secondary overflow-hidden"
									>
										{@render sol.render(st.sel, st.browsed)}
									</div>
								</div>
							{/each}
						</div>
					</section>
				{/each}
			</div>
		</div>
	</div>
{/if}
