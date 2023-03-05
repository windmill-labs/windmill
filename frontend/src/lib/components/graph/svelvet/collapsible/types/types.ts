export interface CollapsibleType {
  id: string;
  nodeId: string;
  hideCount: number;
  state: 'expanded' | 'collapsed';
  isHidden: Function;
  toggleState: Function;
}
