import type { Runnable } from './utils'
import { deepEqual } from 'fast-equals'

/**
 * Snapshot entry containing raw app state at a point in time
 */
export interface HistoryEntry {
	id: number
	timestamp: Date
	files: Record<string, string>
	runnables: Record<string, Runnable>
	summary: string
}

/**
 * A branch in the history tree
 * Contains entries that diverged from a fork point
 */
export interface HistoryBranch {
	id: number
	forkPointId: number // ID of the entry this branch forked from
	entries: HistoryEntry[]
}

/**
 * Configuration for history manager
 */
export interface HistoryConfig {
	maxEntries: number
	autoSnapshotInterval?: number // milliseconds
}

/**
 * History manager for raw apps with branching support
 *
 * Main timeline: The current working branch
 * Branches: Preserved "futures" when navigating to historical points and making changes
 *
 * When selecting a historical entry and making changes:
 * - Current "future" entries become a branch (forked from selected point)
 * - The selected entry becomes the new "head" of main timeline
 *
 * When selecting an entry on a branch and making changes:
 * - That branch becomes the main timeline
 * - The old main timeline (from fork point onwards) becomes a branch
 */
export class RawAppHistoryManager {
	// Main timeline entries
	private entries = $state<HistoryEntry[]>([])
	// Preserved branches (old "futures" that were branched off)
	private branches = $state<HistoryBranch[]>([])
	private autoSnapshotTimer: number | undefined = undefined
	private getStateFn:
		| (() => {
				files: Record<string, string>
				runnables: Record<string, Runnable>
				summary: string
		  })
		| undefined = undefined
	private isCreatingSnapshot = $state(false)
	// Currently selected entry index in main timeline (-1 = at latest/no selection)
	private currentIndex = $state(-1)
	// If viewing a branch, which branch and entry index
	private currentBranchId = $state<number | undefined>(undefined)
	private currentBranchEntryIndex = $state(-1)
	private entryIdCounter = $state(0)
	private branchIdCounter = $state(0)
	// Track if current state has pending changes
	private hasPendingChanges = $state(false)

	// Derived state
	public readonly hasEntries = $derived(this.entries.length > 0)
	public readonly entryCount = $derived(this.entries.length)
	public readonly allEntries = $derived(this.entries.slice())
	public readonly allBranches = $derived(this.branches.slice())
	public readonly canSnapshot = $derived(!this.isCreatingSnapshot)

	// The ID of the currently selected entry (main timeline or branch)
	public readonly selectedEntryId = $derived.by(() => {
		if (this.currentBranchId !== undefined) {
			const branch = this.branches.find((b) => b.id === this.currentBranchId)
			return branch?.entries[this.currentBranchEntryIndex]?.id
		}
		if (this.currentIndex === -1) return undefined
		return this.entries[this.currentIndex]?.id
	})

	// Whether we need to save current state before navigating
	public readonly needsSnapshotBeforeNav = $derived(
		this.currentIndex === -1 && this.currentBranchId === undefined && this.hasPendingChanges
	)

	public readonly canUndo = $derived(
		this.currentIndex > 0 ||
			(this.currentIndex === -1 && this.entries.length > 1) ||
			(this.currentIndex === -1 && this.entries.length === 1 && this.hasPendingChanges)
	)

	public readonly canRedo = $derived(
		this.currentIndex !== -1 && this.currentIndex < this.entries.length - 1
	)

	constructor(private config: HistoryConfig) {}

	/**
	 * Create a snapshot from provided state
	 */
	createSnapshot(
		files: Record<string, string>,
		runnables: Record<string, Runnable>,
		summary: string
	): HistoryEntry {
		return {
			id: this.entryIdCounter++,
			timestamp: new Date(),
			files: structuredClone($state.snapshot(files)),
			runnables: structuredClone($state.snapshot(runnables)),
			summary: $state.snapshot(summary)
		}
	}

	/**
	 * Check if state has changed since last snapshot
	 */
	private hasStateChanged(
		files: Record<string, string>,
		runnables: Record<string, Runnable>,
		summary: string
	): boolean {
		if (this.entries.length === 0) return true

		const lastEntry = this.entries[this.entries.length - 1]
		return (
			!deepEqual(lastEntry.files, files) ||
			!deepEqual(lastEntry.runnables, runnables) ||
			lastEntry.summary !== summary
		)
	}

	/**
	 * Add a snapshot to the main timeline
	 */
	addSnapshot(entry: HistoryEntry): void {
		if (this.isCreatingSnapshot) {
			console.warn('Snapshot already in progress, skipping')
			return
		}

		this.isCreatingSnapshot = true

		try {
			this.entries = [...this.entries, entry]

			// FIFO: Remove oldest entries when exceeding limit
			if (this.entries.length > this.config.maxEntries) {
				const removed = this.entries.slice(0, this.entries.length - this.config.maxEntries)
				this.entries = this.entries.slice(-this.config.maxEntries)
				// Clean up branches that reference removed entries
				const removedIds = new Set(removed.map((e) => e.id))
				this.branches = this.branches.filter((b) => !removedIds.has(b.forkPointId))
			}

			this.hasPendingChanges = false
		} finally {
			this.isCreatingSnapshot = false
		}
	}

	/**
	 * Mark that there are pending changes
	 * When making changes from a historical position, create a branch from the "future"
	 */
	markPendingChanges(): void {
		// If we're on a branch and making changes, that branch becomes main
		if (this.currentBranchId !== undefined) {
			this.promoteBranchToMain()
		}
		// If we're at a historical position on main timeline
		else if (this.currentIndex !== -1 && this.currentIndex < this.entries.length - 1) {
			this.createBranchFromFuture()
		}

		// Clear selection - we're now in a new unsaved state
		this.currentIndex = -1
		this.currentBranchId = undefined
		this.currentBranchEntryIndex = -1

		this.hasPendingChanges = true
	}

	/**
	 * Create a branch from the "future" entries when making changes from historical position
	 */
	private createBranchFromFuture(): void {
		const forkEntry = this.entries[this.currentIndex]
		const futureEntries = this.entries.slice(this.currentIndex + 1)

		if (futureEntries.length > 0) {
			const newBranch: HistoryBranch = {
				id: this.branchIdCounter++,
				forkPointId: forkEntry.id,
				entries: futureEntries
			}
			this.branches = [...this.branches, newBranch]
		}

		// Truncate main timeline to current position
		this.entries = this.entries.slice(0, this.currentIndex + 1)
		this.currentIndex = -1
	}

	/**
	 * Promote current branch to main timeline
	 * The old main timeline (from fork point onwards) becomes a branch
	 */
	private promoteBranchToMain(): void {
		const branch = this.branches.find((b) => b.id === this.currentBranchId)
		if (!branch) return

		// Find fork point in main timeline
		const forkIndex = this.entries.findIndex((e) => e.id === branch.forkPointId)
		if (forkIndex === -1) return

		// Save the current main timeline's "future" as a new branch (if any entries after fork)
		const mainFutureEntries = this.entries.slice(forkIndex + 1)
		if (mainFutureEntries.length > 0) {
			const oldMainBranch: HistoryBranch = {
				id: this.branchIdCounter++,
				forkPointId: branch.forkPointId,
				entries: mainFutureEntries
			}
			this.branches = [...this.branches.filter((b) => b.id !== this.currentBranchId), oldMainBranch]
		} else {
			// Just remove the current branch from branches list
			this.branches = this.branches.filter((b) => b.id !== this.currentBranchId)
		}

		// New main timeline: entries up to fork point + branch entries up to selected index
		const branchEntriesUpToSelection = branch.entries.slice(0, this.currentBranchEntryIndex + 1)
		this.entries = [...this.entries.slice(0, forkIndex + 1), ...branchEntriesUpToSelection]

		// Reset selection state
		this.currentBranchId = undefined
		this.currentBranchEntryIndex = -1
		this.currentIndex = -1
	}

	getId(): number {
		return this.entryIdCounter
	}

	/**
	 * Manually create and add a snapshot
	 * @param force - If true, create snapshot even if state hasn't changed
	 */
	manualSnapshot(
		files: Record<string, string>,
		runnables: Record<string, Runnable>,
		summary: string,
		force = false
	): HistoryEntry | undefined {
		if (!force && !this.hasStateChanged(files, runnables, summary)) {
			return
		}

		const entry = this.createSnapshot(files, runnables, summary)
		this.addSnapshot(entry)
		return entry
	}

	/**
	 * Start automatic snapshot timer
	 */
	startAutoSnapshot(
		getState: () => {
			files: Record<string, string>
			runnables: Record<string, Runnable>
			summary: string
		}
	): void {
		this.stopAutoSnapshot()
		this.getStateFn = getState

		if (!this.config.autoSnapshotInterval) return

		this.autoSnapshotTimer = setInterval(() => {
			if (this.getStateFn && this.currentIndex === -1 && this.currentBranchId === undefined) {
				const { files, runnables, summary } = this.getStateFn()
				this.manualSnapshot(files, runnables, summary)
			}
		}, this.config.autoSnapshotInterval) as unknown as number
	}

	/**
	 * Stop automatic snapshot timer
	 */
	stopAutoSnapshot(): void {
		if (this.autoSnapshotTimer !== undefined) {
			clearInterval(this.autoSnapshotTimer)
			this.autoSnapshotTimer = undefined
		}
	}

	/**
	 * Select an entry (on main timeline or a branch)
	 * If there are pending changes, a snapshot should be created first by the caller
	 */
	selectEntry(id: number): HistoryEntry | undefined {
		// Check main timeline first
		const mainIndex = this.entries.findIndex((e) => e.id === id)
		if (mainIndex !== -1) {
			this.currentIndex = mainIndex
			this.currentBranchId = undefined
			this.currentBranchEntryIndex = -1
			this.hasPendingChanges = false
			return this.entries[mainIndex]
		}

		// Check branches
		for (const branch of this.branches) {
			const branchIndex = branch.entries.findIndex((e) => e.id === id)
			if (branchIndex !== -1) {
				this.currentBranchId = branch.id
				this.currentBranchEntryIndex = branchIndex
				this.currentIndex = -1
				this.hasPendingChanges = false
				return branch.entries[branchIndex]
			}
		}

		return undefined
	}

	/**
	 * Clear selection (go back to latest state)
	 */
	clearSelection(): void {
		this.currentIndex = -1
		this.currentBranchId = undefined
		this.currentBranchEntryIndex = -1
	}

	/**
	 * Get entry by ID (searches main timeline and branches)
	 */
	getEntryById(id: number): HistoryEntry | undefined {
		const mainEntry = this.entries.find((e) => e.id === id)
		if (mainEntry) return mainEntry

		for (const branch of this.branches) {
			const branchEntry = branch.entries.find((e) => e.id === id)
			if (branchEntry) return branchEntry
		}

		return undefined
	}

	/**
	 * Get branch that contains an entry
	 */
	getBranchForEntry(id: number): HistoryBranch | undefined {
		return this.branches.find((b) => b.entries.some((e) => e.id === id))
	}

	/**
	 * Undo to previous state
	 */
	undo(): HistoryEntry | null {
		if (!this.canUndo) return null

		if (this.currentIndex === -1) {
			this.currentIndex = this.entries.length - 2
		} else {
			this.currentIndex--
		}

		this.hasPendingChanges = false
		return this.entries[this.currentIndex]
	}

	/**
	 * Redo to next state
	 */
	redo(): HistoryEntry | null {
		if (!this.canRedo) return null

		this.currentIndex++
		this.hasPendingChanges = false
		return this.entries[this.currentIndex]
	}

	/**
	 * Clear all history
	 */
	clearHistory(): void {
		this.entries = []
		this.branches = []
		this.currentIndex = -1
		this.currentBranchId = undefined
		this.currentBranchEntryIndex = -1
	}

	/**
	 * Cleanup resources
	 */
	destroy(): void {
		this.stopAutoSnapshot()
		this.clearHistory()
	}
}
