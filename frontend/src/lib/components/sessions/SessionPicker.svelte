<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		ChevronDown,
		ChevronRight,
		EllipsisVertical,
		MessageSquare,
		Pencil,
		Plus,
		Trash2
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { goto } from '$lib/navigation'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { slide } from 'svelte/transition'
	import {
		createSession,
		getEffectiveWorkspaceId,
		renameSession,
		selectSession,
		sessionState,
		syncWorkspaceTo,
		type Session
	} from './sessionState.svelte'
	import {
		getOrCreateRuntime,
		getRuntime,
		getSessionChatStatus,
		removeSession
	} from './sessionRuntime.svelte'
	import SessionStatusDot from './SessionStatusDot.svelte'
	import { Menu, Menubar, MenuItem } from '$lib/components/meltComponents'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { visibleWorkspaceIds } from './sessionScope.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { userWorkspaces } from '$lib/stores'

	// A session is "in a fork" when its effective workspace has a parent
	// workspace in the user's list — i.e., it's not the root.
	function sessionIsFork(session: Session): boolean {
		const ws = getEffectiveWorkspaceId(session)
		if (!ws) return false
		const entry = $userWorkspaces.find((w) => w.id === ws)
		return !!entry?.parent_workspace_id
	}

	// Sessions piggyback on the same dev gate as the global AI chat — when
	// the feature flag is off, the sidebar section is hidden entirely.
	const globalEnabled = isGlobalAiEnabled()

	interface Props {
		isCollapsed?: boolean
	}

	let { isCollapsed = false }: Props = $props()

	const sectionCollapsed = useLocalStorageValue(
		'windmill_sessions_section_collapsed',
		false,
		'boolean'
	)

	let listRoot: HTMLDivElement | undefined = $state()

	// Sessions visible in the current workspace (active workspace + its
	// forks). Drafts (no committed workspace) are scoped by their
	// pending workspace pick — set at create time to the workspace the
	// user was in.
	const visibleSessions = $derived(
		sessionState.sessions.filter((s) => {
			const ws = getEffectiveWorkspaceId(s)
			return ws ? $visibleWorkspaceIds.has(ws) : false
		})
	)

	// Eagerly create a runtime per VISIBLE session so the status dot reflects
	// the persisted chat (last message, pending confirmation, etc.) without
	// requiring the user to open the session first. Sessions outside the
	// current workspace scope are left cold to avoid opening IDB connections
	// for unrelated work.
	$effect(() => {
		for (const session of visibleSessions) {
			getOrCreateRuntime(session)
		}
	})

	async function activate(session: Session, restoreFocus: boolean = false) {
		selectSession(session.id)
		// If the session has a committed workspace different from the
		// active one, switch globally so the editor/forks resolve correctly.
		syncWorkspaceTo(session.workspace_id)
		// Refresh the fork diff count — users typically click back into a
		// session after editing items elsewhere in the SPA, where neither
		// the visibility-change nor the AI-loading signal would fire.
		void getRuntime(session.id)?.refreshForkComparison()
		await goto(`/sessions?session_name=${encodeURIComponent(session.name)}`)
		if (restoreFocus) {
			// goto() resets focus to <body> — put it back on the active session button
			// so subsequent arrow keys keep navigating the list.
			requestAnimationFrame(() => {
				const selected = listRoot?.querySelector<HTMLButtonElement>(
					'button[data-session-button][aria-selected="true"]'
				)
				selected?.focus()
			})
		}
	}

	async function createAndOpen() {
		await activate(createSession())
	}

	let editingId: string | undefined = $state(undefined)
	let renameDraft = $state('')

	function startRename(session: Session) {
		editingId = session.id
		renameDraft = session.summary ?? ''
	}

	function commitRename() {
		const id = editingId
		if (!id) return
		renameSession(id, renameDraft)
		editingId = undefined
	}

	function cancelRename() {
		editingId = undefined
	}

	async function confirmDelete(session: Session) {
		const label = session.summary ?? session.name
		if (!window.confirm(`Delete session "${label}"? This cannot be undone.`)) return
		const wasActive = sessionState.currentSessionId === session.id
		removeSession(session.id)
		if (wasActive) {
			const next = sessionState.sessions[0]
			if (next) await activate(next)
			else await goto('/sessions')
		}
	}

	function focusAt(index: number) {
		const buttons = listRoot
			? Array.from(listRoot.querySelectorAll<HTMLButtonElement>('button[data-session-button]'))
			: []
		if (buttons.length === 0) return
		const wrapped = ((index % buttons.length) + buttons.length) % buttons.length
		buttons[wrapped]?.focus()
	}

	function handleListKeydown(e: KeyboardEvent) {
		if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') {
			return
		}
		const buttons = listRoot
			? Array.from(listRoot.querySelectorAll<HTMLButtonElement>('button[data-session-button]'))
			: []
		if (buttons.length === 0) return
		const current = buttons.indexOf(document.activeElement as HTMLButtonElement)
		e.preventDefault()
		if (e.key === 'ArrowDown') focusAt(current < 0 ? 0 : current + 1)
		else if (e.key === 'ArrowUp') focusAt(current < 0 ? buttons.length - 1 : current - 1)
		else if (e.key === 'Home') focusAt(0)
		else if (e.key === 'End') focusAt(buttons.length - 1)
	}

	const menuItemBase = twMerge(
		'text-secondary text-left font-normal text-xs',
		'flex flex-row items-center gap-2 px-3 py-1.5 w-full',
		'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)
</script>

{#if !globalEnabled}
	<!-- Sessions hidden until the global-ai dev gate is enabled. -->
{:else if isCollapsed}
	<div class="px-2 pt-3 pb-2 border-b border-light dark:border-gray-700">
		<Menubar>
			{#snippet children({ createMenu })}
				<Menu {createMenu} usePointerDownOutside>
					{#snippet triggr({ trigger })}
						<MenuButton
							class="!text-xs"
							icon={MessageSquare}
							label="Sessions"
							{isCollapsed}
							{trigger}
						/>
					{/snippet}
					{#snippet children({ item })}
						<div class="divide-y min-w-48" role="none">
							<div class="py-1" role="none">
								<MenuItem class={menuItemBase} onClick={createAndOpen} {item}>
									<Plus size={14} />
									New session
								</MenuItem>
							</div>
							<div class="py-1" role="none">
								{#each visibleSessions as session (session.id)}
									{@const runtime = getRuntime(session.id)}
									{@const status = runtime ? getSessionChatStatus(runtime) : 'idle'}
									{@const isSelected = session.id === sessionState.currentSessionId}
									<MenuItem
										class={twMerge(menuItemBase, isSelected ? 'bg-surface-hover' : '')}
										onClick={() => activate(session)}
										{item}
									>
										<SessionStatusDot {status} isFork={sessionIsFork(session)} />
										<span class="truncate flex-1 text-left">
											{session.summary ?? 'Untitled session'}
										</span>
									</MenuItem>
								{/each}
							</div>
						</div>
					{/snippet}
				</Menu>
			{/snippet}
		</Menubar>
	</div>
{:else}
	<div class="px-2 pt-3 pb-2 flex flex-col gap-1 border-b border-light dark:border-gray-700">
		<div class="flex flex-row items-center justify-between pl-1 pr-0.5">
			<button
				type="button"
				onclick={() => (sectionCollapsed.val = !sectionCollapsed.val)}
				class="text-secondary text-[0.5rem] uppercase flex flex-row items-center gap-1 rounded px-1 -mx-1 py-0.5 hover:bg-surface-hover focus:outline-none"
				aria-expanded={!sectionCollapsed.val}
			>
				Sessions
				{#if sectionCollapsed.val}
					<ChevronRight size={10} />
				{:else}
					<ChevronDown size={10} />
				{/if}
			</button>
			<Button
				variant="subtle"
				size="xs2"
				iconOnly
				startIcon={{ icon: Plus }}
				on:click={createAndOpen}
				title="New session"
			/>
		</div>
		{#if !sectionCollapsed.val}
			<div
				bind:this={listRoot}
				transition:slide={{ duration: 180 }}
				class="flex flex-col gap-0.5 max-h-[40vh] overflow-y-auto"
				onkeydown={handleListKeydown}
				role="listbox"
				tabindex="-1"
			>
				{#each visibleSessions as session (session.id)}
					{@const runtime = getRuntime(session.id)}
					{@const status = runtime ? getSessionChatStatus(runtime) : 'idle'}
					{@const isSelected = session.id === sessionState.currentSessionId}
					{@const isEditing = editingId === session.id}
					<div
						class={twMerge(
							'flex flex-row items-center group rounded',
							isSelected ? 'bg-surface-hover text-primary' : 'hover:bg-surface-hover'
						)}
					>
						{#if isEditing}
							<span class="flex flex-row items-center gap-2 flex-1 px-2 py-1 min-w-0">
								<SessionStatusDot {status} isFork={sessionIsFork(session)} />
								<!-- svelte-ignore a11y_autofocus -->
								<input
									type="text"
									bind:value={renameDraft}
									onkeydown={(e) => {
										if (e.key === 'Enter') commitRename()
										else if (e.key === 'Escape') cancelRename()
									}}
									onblur={commitRename}
									placeholder="Untitled session"
									autofocus
									spellcheck="false"
									class="flex-1 min-w-0 bg-transparent border-0 outline-none text-xs font-normal text-primary"
								/>
							</span>
						{:else}
							<button
								type="button"
								data-session-button
								role="option"
								aria-selected={isSelected}
								onclick={() => activate(session)}
								class="flex flex-row items-center gap-2 text-left text-xs font-normal text-secondary focus:outline-none flex-1 min-w-0 px-2 py-1"
							>
								<SessionStatusDot {status} isFork={sessionIsFork(session)} />
								<span class="truncate flex-1">{session.summary ?? 'Untitled session'}</span>
							</button>
							<div
								class="opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity pr-0.5"
							>
								<DropdownV2
									fixedHeight={false}
									placement="bottom-end"
									items={[
										{
											displayName: 'Rename',
											icon: Pencil,
											action: () => startRename(session)
										},
										{
											displayName: 'Delete',
											icon: Trash2,
											type: 'delete',
											action: () => confirmDelete(session)
										}
									]}
								>
									{#snippet buttonReplacement()}
										<span
											class="inline-flex items-center justify-center w-5 h-5 rounded text-tertiary hover:bg-surface-hover hover:text-primary"
											title="More"
										>
											<EllipsisVertical size={14} />
										</span>
									{/snippet}
								</DropdownV2>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}
