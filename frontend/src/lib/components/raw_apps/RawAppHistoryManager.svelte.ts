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
 * Configuration for history manager
 */
export interface HistoryConfig {
	maxEntries: number
	autoSnapshotInterval?: number // milliseconds
}

/**
 * History manager for raw apps
 * Maintains in-memory circular buffer of app state snapshots
 * Supports auto-snapshots, manual snapshots, preview, and restore
 */
export class RawAppHistoryManager {
	private entries = $state<HistoryEntry[]>([])
	private previewEntry = $state<HistoryEntry | undefined>(undefined)
	private autoSnapshotTimer: number | undefined = undefined
	private getStateFn:
		| (() => {
				files: Record<string, string>
				runnables: Record<string, Runnable>
				summary: string
		  })
		| undefined = undefined
	private isCreatingSnapshot = $state(false) // Prevents concurrent snapshot operations
	private currentIndex = $state(-1) // -1 means viewing latest, used for undo/redo
	private id = $state(0)
	// Track if current state has pending changes (differs from last snapshot)
	private hasPendingChanges = $state(false)
	// Derived state
	public readonly hasEntries = $derived(this.entries.length > 0)
	public readonly entryCount = $derived(this.entries.length)
	public readonly isPreviewMode = $derived(this.previewEntry !== undefined)
	public readonly allEntries = $derived(this.entries.slice())
	public readonly currentPreview = $derived(this.previewEntry)
	public readonly canSnapshot = $derived(!this.isCreatingSnapshot)
	public readonly canUndo = $derived(
		this.currentIndex > 0 ||
			(this.currentIndex === -1 && this.entries.length > 1) ||
			(this.currentIndex === -1 && this.entries.length === 1 && this.hasPendingChanges)
	)
	public readonly canRedo = $derived(
		this.currentIndex !== -1 && this.currentIndex < this.entries.length - 1
	)

	constructor(private config: HistoryConfig) {
		this.id = 0
	}

	/**
	 * Create a snapshot from provided state
	 * Uses structuredClone for deep cloning to avoid reactive proxy issues
	 */
	createSnapshot(
		files: Record<string, string>,
		runnables: Record<string, Runnable>,
		summary: string
	): HistoryEntry {
		return {
			id: this.id++,
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
	 * Add a snapshot to history
	 * Enforces FIFO when exceeding maxEntries
	 * Uses guard to prevent concurrent operations
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
				this.entries = this.entries.slice(-this.config.maxEntries)
			}

			// Reset pending changes since we just saved
			this.hasPendingChanges = false
		} finally {
			this.isCreatingSnapshot = false
		}
	}

	/**
	 * Mark that there are pending changes (current state differs from last snapshot)
	 * This enables the undo button even when there's only one snapshot
	 */
	markPendingChanges(): void {
		this.hasPendingChanges = true
	}

	getId(): number {
		return this.id
	}
	/**
	 * Manually create and add a snapshot
	 * Only creates if state has changed
	 */
	manualSnapshot(
		files: Record<string, string>,
		runnables: Record<string, Runnable>,
		summary: string
	): HistoryEntry | undefined {
		if (!this.hasStateChanged(files, runnables, summary)) {
			return // No changes, don't create snapshot
		}

		const entry = this.createSnapshot(files, runnables, summary)
		this.addSnapshot(entry)
		return entry
	}

	/**
	 * Start automatic snapshot timer
	 * Only creates snapshots when state has actually changed
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
			if (!this.isPreviewMode && this.getStateFn) {
				const { files, runnables, summary } = this.getStateFn()
				// Only snapshot if state has changed
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
	 * Set preview mode to view a specific entry
	 */
	setPreview(id: number): void {
		const entry = this.getEntryById(id)
		if (entry) {
			this.previewEntry = entry
		}
	}

	/**
	 * Clear preview mode
	 */
	clearPreview(): void {
		this.previewEntry = undefined
	}

	/**
	 * Get entry at specific index
	 */
	getEntry(index: number): HistoryEntry | undefined {
		return this.entries[index]
	}

	getEntryById(id: number): HistoryEntry | undefined {
		return this.entries.find((e) => e.id === id)
	}

	setSelectedEntry(id: number): HistoryEntry | undefined {
		console.log('setting selected entry', id)
		const index = this.entries.findIndex((e) => e.id === id)
		if (index === -1) {
			return
		}
		this.currentIndex = index
		this.previewEntry = this.entries[index]
		return this.previewEntry
	}

	/**
	 * Clear all history
	 */
	clearHistory(): void {
		this.entries = []
		this.previewEntry = undefined
	}

	/**
	 * Undo to previous state
	 * Returns the entry to restore, or null if can't undo
	 */
	undo(): HistoryEntry | null {
		if (!this.canUndo) return null

		if (this.currentIndex === -1) {
			// Currently viewing latest, move to second-to-last
			this.currentIndex = this.entries.length - 2
		} else {
			// Move back one step
			this.currentIndex--
		}

		return this.entries[this.currentIndex]
	}

	/**
	 * Redo to next state
	 * Returns the entry to restore, or null if can't redo
	 */
	redo(): HistoryEntry | null {
		if (!this.canRedo) return null

		this.currentIndex++

		// If we've reached the latest entry, reset index to -1
		if (this.currentIndex === this.entries.length - 1) {
			this.currentIndex = -1
		}

		return this.entries[this.currentIndex]
	}

	/**
	 * Cleanup resources
	 */
	destroy(): void {
		this.stopAutoSnapshot()
		this.clearHistory()
	}
}
