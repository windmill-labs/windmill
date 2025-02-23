import { type editor as meditor } from 'monaco-editor'

class AIChat {
    editor: meditor.IStandaloneCodeEditor


    lineChanges: {
        startLineNumber: number,
        endLineNumber: number,
        value: string
    }[] = []

    constructor(editor: meditor.IStandaloneCodeEditor) {
        this.editor = editor
    }


}