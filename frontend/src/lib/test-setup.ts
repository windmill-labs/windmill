/**
 * Vitest setup file to mock browser globals for testing
 */

import { vi } from 'vitest'

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

vi.mock('@codingame/monaco-vscode-standalone-typescript-language-features/worker', () => ({
	TypeScriptWorker: class TypeScriptWorker {
		private _mockScriptSnapshot?: {
			getText: (start: number, end: number) => string
			getLength: () => number
		}

		setMockCode(t: string) {
			this._mockScriptSnapshot = {
				getText: (start: number, end: number) => t.substring(start, end),
				getLength: () => t.length
			}
		}

		getScriptSnapshot(_fileName: string) {
			return this._mockScriptSnapshot
		}
		getScriptVersion(_fileName: string) {
			return '1'
		}
	},
	ts: undefined,
	initialize: undefined
}))
