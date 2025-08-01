"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const YAML = require("../src/");
class AbstractVisitor {
    accept(node) {
        switch (node.kind) {
            case YAML.Kind.SCALAR: {
                return this.visitScalar(node);
            }
            case YAML.Kind.MAP: {
                return this.visitMap(node);
            }
            case YAML.Kind.MAPPING: {
                return this.visitMapping(node);
            }
            case YAML.Kind.SEQ: {
                return this.visitSequence(node);
            }
            case YAML.Kind.ANCHOR_REF: {
                return this.visitAnchorRef(node);
            }
            case YAML.Kind.INCLUDE_REF: {
                return this.visitIncludeRef(node);
            }
        }
        throw new Error(`Kind, ${node.kind} not implemented.`);
    }
}
exports.AbstractVisitor = AbstractVisitor;
//# sourceMappingURL=visitor.js.map