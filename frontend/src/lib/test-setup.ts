/**
 * Vitest setup file to mock browser globals for testing
 */

// Mock localStorage
const localStorageMock = {
	store: {} as Record<string, string>,
	getItem(key: string) {
		return this.store[key] ?? null
	},
	setItem(key: string, value: string) {
		this.store[key] = value
	},
	removeItem(key: string) {
		delete this.store[key]
	},
	clear() {
		this.store = {}
	},
	get length() {
		return Object.keys(this.store).length
	},
	key(index: number) {
		return Object.keys(this.store)[index] ?? null
	}
}

// Mock sessionStorage
const sessionStorageMock = {
	store: {} as Record<string, string>,
	getItem(key: string) {
		return this.store[key] ?? null
	},
	setItem(key: string, value: string) {
		this.store[key] = value
	},
	removeItem(key: string) {
		delete this.store[key]
	},
	clear() {
		this.store = {}
	},
	get length() {
		return Object.keys(this.store).length
	},
	key(index: number) {
		return Object.keys(this.store)[index] ?? null
	}
}

// Assign to globalThis
Object.defineProperty(globalThis, 'localStorage', {
	value: localStorageMock,
	writable: true
})

Object.defineProperty(globalThis, 'sessionStorage', {
	value: sessionStorageMock,
	writable: true
})