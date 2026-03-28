import { SchemaProperty } from "./common.ts";

export interface FlowDefinition {
    summary: string,
    description: string,
    value: {
        modules: any[]
    },
    schema: {
        $schema: string,
        type: string,
        order: string[],
        properties: { [name: string]: SchemaProperty},
        required: string[]
    }
}

export function defaultFlowDefinition(): FlowDefinition {
    return {
        summary: '',
        description: '',
        value: { 
            modules: []
        },
        schema: {
            $schema: 'https://json-schema.org/draft/2020-12/schema',
            type: 'object',
            order: [],
            properties: {},
            required: []
        },
    }
}
