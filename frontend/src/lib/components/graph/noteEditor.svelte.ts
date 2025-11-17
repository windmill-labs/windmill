import type { FlowNote } from '$lib/gen'
import type { StateStore } from '$lib/utils'
import type { ExtendedOpenFlow } from '../flows/types'
import type { NoteColor } from './noteColors'
import { generateId } from './util'
import { getContext, setContext } from 'svelte'

/**
 * Utility class for editing flow notes via direct flowStore mutations
 * This class is designed to be used in editor contexts via Svelte context
 */
export class NoteEditor {
	private flowStore: StateStore<ExtendedOpenFlow>

	constructor(flowStore: StateStore<ExtendedOpenFlow>) {
		this.flowStore = flowStore
	}

	/**
	 * Get the current notes array from the flow store
	 */
	private getNotes(): FlowNote[] {
		return this.flowStore.val.value?.notes || []
	}

	/**
	 * Set the notes array in the flow store
	 */
	private setNotes(notes: FlowNote[]): void {
		if (this.flowStore.val.value) {
			this.flowStore.val.value.notes = notes
		}
	}

	/**
	 * Add a new note to the flow
	 */
	addNote(note: Omit<FlowNote, 'id'>): string {
		const notes = this.getNotes()
		const newNote: FlowNote = {
			id: generateId(),
			...note
		}
		this.setNotes([...notes, newNote])
		return newNote.id
	}

	/**
	 * Update the text content of a note
	 */
	updateText(noteId: string, text: string): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, text } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Update the color of a note
	 */
	updateColor(noteId: string, color: NoteColor): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, color } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Update the position of a note
	 */
	updatePosition(noteId: string, position: { x: number; y: number }): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, position } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Update the size of a note
	 */
	updateSize(noteId: string, size: { width: number; height: number }): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, size } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Toggle the locked state of a note
	 */
	updateLock(noteId: string, locked: boolean): void {
		const notes = this.getNotes()
		const updatedNotes = notes.map((note) => (note.id === noteId ? { ...note, locked } : note))
		this.setNotes(updatedNotes)
	}

	/**
	 * Delete a note from the flow
	 */
	deleteNote(noteId: string): void {
		const notes = this.getNotes()
		const updatedNotes = notes.filter((note) => note.id !== noteId)
		this.setNotes(updatedNotes)
	}

	/**
	 * Create a group note containing the specified node IDs
	 */
	createGroupNote(
		nodeIds: string[],
		text: string = '### Group note\nDouble click to edit me'
	): string {
		// Position and size will be calculated dynamically by layout

		const groupNote: Omit<FlowNote, 'id'> = {
			text,
			color: 'blue', // Default color, can be made configurable
			type: 'group',
			contained_node_ids: nodeIds,
			locked: false
		}

		return this.addNote(groupNote)
	}

	/**
	 * Check if editing is available (flowStore is properly initialized)
	 */
	isAvailable(): boolean {
		return !!this.flowStore.val.value
	}
}

/**
 * Context type for NoteEditor
 */
export type NoteEditorContext = {
	noteEditor: NoteEditor
}

const CONTEXT_KEY = 'NoteEditorContext'

/**
 * Set the NoteEditor context (used in FlowBuilder)
 */
export function setNoteEditorContext(noteEditor: NoteEditor): void {
	setContext<NoteEditorContext>(CONTEXT_KEY, { noteEditor })
}

/**
 * Get the NoteEditor context (used in components that need editing capabilities)
 */
export function getNoteEditorContext(): NoteEditorContext | undefined {
	return getContext<NoteEditorContext | undefined>(CONTEXT_KEY)
}
