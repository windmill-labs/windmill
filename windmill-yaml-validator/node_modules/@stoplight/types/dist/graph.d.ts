import { IExternalDocs } from './http-spec';
export interface IShareableNode {
    id: string;
}
export interface ISpecExtensions {
    extensions?: Extensions;
}
export interface IComponentNode {
    key: string;
}
export interface INode extends ISpecExtensions, IShareableNode {
    /** An internal identifier. For example, the operationId property in OAS. */
    iid?: string;
    tags?: INodeTag[];
    summary?: string;
    description?: string;
}
export interface INodeTag extends IShareableNode, ISpecExtensions {
    name: string;
    description?: string;
    /** only applicable to tag _definitions_, not references */
    externalDocs?: IExternalDocs;
}
export interface INodeVariable extends ISpecExtensions {
    default: string;
    description?: string;
    enum?: string[];
}
interface INodeExampleBase extends ISpecExtensions {
    key: string;
    summary?: string;
    description?: string;
}
export interface INodeExample extends INodeExampleBase, IShareableNode {
    value: unknown;
}
export interface INodeExternalExample extends INodeExampleBase, IShareableNode {
    externalValue: string;
}
export interface Extensions {
    [key: string]: unknown;
}
export {};
