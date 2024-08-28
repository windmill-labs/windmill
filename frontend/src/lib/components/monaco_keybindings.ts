import type { IDisposable } from 'monaco-editor'
import * as monaco from 'monaco-editor'
// @ts-ignore
import * as MonacoVim from 'monaco-vim'

export function initVim(
	editor: monaco.editor.ICodeEditor,
	statusBarElement: Element,
	save: () => void
): IDisposable {
	MonacoVim.VimMode.Vim.defineEx('write', 'w', function () {
		save()
	})
	return MonacoVim.initVimMode(editor, statusBarElement)
}
