/**
 * The attachment work a composer has running: the reads that decode a file, and the routing
 * of a drop that will start one. A host taking the composer's draft as its panel goes waits
 * on this, so a file still being read is handed over rather than lost with the composer.
 */
export class ReadsInFlight {
	#running = new Set<Promise<unknown>>()

	/** Whether anything is running; read before a draft is taken. */
	get busy(): boolean {
		return this.#running.size > 0
	}

	/** Runs a read, counted until it settles. */
	track<T>(run: () => Promise<T>): Promise<T> {
		const read = run()
		this.#running.add(read)
		void read.catch(() => {}).finally(() => this.#running.delete(read))
		return read
	}

	/**
	 * Counts the routing of a drop, which resolves file handles before it can start a read.
	 * The returned function ends it; calling it twice ends it once.
	 */
	hold(): () => void {
		let finish!: () => void
		const routing = new Promise<void>((resolve) => (finish = resolve))
		this.#running.add(routing)
		void routing.finally(() => this.#running.delete(routing))
		return finish
	}

	/**
	 * Settles once nothing is running. Waiting once is not enough: routing ends in a read
	 * that starts only as the routing finishes, so each wait can uncover the next.
	 */
	async settled(): Promise<void> {
		while (this.#running.size > 0) {
			await Promise.allSettled([...this.#running])
		}
	}
}
