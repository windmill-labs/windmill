"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.FlowValidator = void 0;
const ajv_1 = __importDefault(require("ajv"));
const yaml_1 = require("@stoplight/yaml");
const openflow_json_1 = __importDefault(require("../gen/openflow.json"));
/**
 * Flow validator class that initializes AJV once and reuses it for validation.
 */
class FlowValidator {
    validate;
    constructor() {
        const ajv = new ajv_1.default({ strict: false, allErrors: true, discriminator: true });
        for (const [n, s] of Object.entries(openflow_json_1.default.components.schemas)) {
            ajv.addSchema(s, `#/components/schemas/${n}`);
        }
        this.validate = ajv.getSchema('#/components/schemas/OpenFlow');
    }
    /**
     * Validates a flow document against the OpenFlow schema.
     * @param doc - The YAML flow document as a string
     * @returns Object containing the parsed document and any validation errors
     */
    validateFlow(doc) {
        if (typeof doc !== 'string') {
            throw new Error('Document must be a string');
        }
        const parsed = (0, yaml_1.parseWithPointers)(doc);
        const { data } = parsed;
        const ok = this.validate(data);
        if (ok) {
            return {
                parsed,
                errors: [],
            };
        }
        return {
            parsed,
            errors: this.validate.errors,
        };
    }
}
exports.FlowValidator = FlowValidator;
