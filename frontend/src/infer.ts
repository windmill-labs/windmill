import type { Schema, SchemaProperty } from "./common"
import { ScriptService } from "./gen"
import { sendUserToast } from "./utils"

export async function inferArgs(code: string, schema: Schema): Promise<void> {
    try {
        const inferedSchema = await ScriptService.toJsonschema({
            requestBody: code
        })
        schema.required = []
        const oldProperties = Object.assign({}, schema.properties)
        schema.properties = {}

        for (const arg of inferedSchema.args) {
            if (!(arg.name in oldProperties)) {
                schema.properties[arg.name] = { description: '', type: '' }
            } else {
                schema.properties[arg.name] = oldProperties[arg.name]
            }
            pythonToJsonSchemaType(arg.typ, schema.properties[arg.name])
            schema.properties[arg.name].default = arg.default

            if (!arg.has_default) {
                schema.required.push(arg.name)
            }
        }
    } catch (err) {
        console.error(err)
        sendUserToast(`Could not infer schema: ${err.body ?? err}`, true)
    }
}

function array_move<T>(arr: T[], fromIndex: number, toIndex: number) {
    var element = arr[fromIndex]
    arr.splice(fromIndex, 1)
    arr.splice(toIndex, 0, element)
}

function pythonToJsonSchemaType(t: string, s: SchemaProperty): void {
    if (t === 'int') {
        s.type = 'integer'
    } else if (t === 'float') {
        s.type = 'number'
    } else if (t === 'bool') {
        s.type = 'boolean'
    } else if (t === 'str') {
        s.type = 'string'
    } else if (t === 'dict') {
        s.type = 'object'
    } else if (t === 'list') {
        s.type = 'array'
    } else if (t === 'bytes') {
        s.type = 'string'
        s.contentEncoding = 'base64'
    } else if (t === 'datetime') {
        s.type = 'string'
        s.format = 'date-time'
    } else {
        s.type = undefined
    }
}
