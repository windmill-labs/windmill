/**
 * What a row can only learn from the job behind it, fetched once and kept while mounted:
 * empty until it lands, and empty for good when the read fails, so a purged or unreadable
 * job is not re-fetched on every render of a streaming transcript.
 */

/**
 * Fetches allowed out at once. A page of conversation rows asks for all of its jobs in the
 * same render, and the answers only fill chips in below text that is already on screen.
 */
const MAX_CONCURRENT = 6

export class JobBackedStore<T> {
	#workspace: () => string | undefined
	#load: (workspace: string, jobId: string) => Promise<T>
	#empty: T
	#byJob = $state<Record<string, T>>({})
	#inFlight = new Set<string>()
	#waiting: string[] = []
	#running = 0

	constructor(
		workspace: () => string | undefined,
		empty: T,
		load: (workspace: string, jobId: string) => Promise<T>
	) {
		this.#workspace = workspace
		this.#empty = empty
		this.#load = load
	}

	/** What the job holds, fetching on first ask. Empty until it lands. */
	get(jobId: string | null | undefined): T {
		if (!jobId) return this.#empty
		const cached = this.#byJob[jobId]
		if (cached) return cached
		this.#enqueue(jobId)
		return this.#empty
	}

	#enqueue(jobId: string) {
		if (this.#inFlight.has(jobId)) return
		this.#inFlight.add(jobId)
		this.#waiting.push(jobId)
		this.#pump()
	}

	#pump() {
		while (this.#running < MAX_CONCURRENT && this.#waiting.length > 0) {
			const jobId = this.#waiting.shift()!
			this.#running++
			void this.#fetch(jobId).finally(() => {
				this.#running--
				this.#pump()
			})
		}
	}

	async #fetch(jobId: string) {
		const workspace = this.#workspace()
		if (!workspace) {
			// Neither cached nor in flight, so the row asks again once a workspace is known.
			this.#inFlight.delete(jobId)
			return
		}
		try {
			this.#byJob = { ...this.#byJob, [jobId]: await this.#load(workspace, jobId) }
		} catch {
			// A purged job, or one this user cannot read: the row keeps what it stored.
			this.#byJob = { ...this.#byJob, [jobId]: this.#empty }
		} finally {
			this.#inFlight.delete(jobId)
		}
	}
}
