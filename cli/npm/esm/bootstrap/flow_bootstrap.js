export function defaultFlowDefinition() {
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
        ws_error_handler_muted: false,
    };
}
