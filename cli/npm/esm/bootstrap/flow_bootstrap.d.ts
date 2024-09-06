import { SchemaProperty } from "./common.js";
export interface FlowDefinition {
    summary: string;
    description: string;
    value: {
        modules: any[];
    };
    schema: {
        $schema: string;
        type: string;
        order: string[];
        properties: {
            [name: string]: SchemaProperty;
        };
        required: string[];
    };
    ws_error_handler_muted: false;
}
export declare function defaultFlowDefinition(): FlowDefinition;
//# sourceMappingURL=flow_bootstrap.d.ts.map