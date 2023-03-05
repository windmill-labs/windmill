/**
 * This model implements functionality for nodes to expand and collapse their children
 */

import type { CollapsibleType } from '../types/types';

/** Class that implements collapsible/expandable functionality for Node objects
 * @param {string} id Unique string that serves as a primary key
 * @param {string} nodeId Foreign key to a Node Object
 */
export class Collapsible implements CollapsibleType {
  constructor(
    public id: string,
    public nodeId: string,
    public hideCount: number,
    public state: 'expanded' | 'collapsed'
  ) {}

  isHidden() {
    return this.hideCount > 0;
  }

  toggleState() {
    this.state = this.state === 'expanded' ? 'collapsed' : 'expanded';
  }
}
