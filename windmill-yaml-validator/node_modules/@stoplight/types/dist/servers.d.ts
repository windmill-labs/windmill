import { Dictionary } from './basic';
import { INodeVariable, IShareableNode } from './graph';
export interface IServer extends IShareableNode {
    url: string;
    name?: string;
    description?: string;
    variables?: Dictionary<INodeVariable, string>;
}
