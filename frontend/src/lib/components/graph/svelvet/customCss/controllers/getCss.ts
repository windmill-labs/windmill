// Required for "custom css" feature

import type { NodeType, StoreType } from '../../store/types/types';

// WHAT: For a given node with a user-defined classname, see if there are any css properties for height and width
//       Get that height and width and set properties in node store. This is necessary for
export const forceCssHeightAndWidth = (store: StoreType, node: NodeType) => {
  let width, height, innerText;
  // Look through each CSS rule to find the one the user defined
  for (let i = 0; i < document.styleSheets.length; i++) {
    const styleSheet = document.styleSheets[i];
    const styleRules = styleSheet.cssRules;
    for (let j = 0; j < styleRules.length; j++) {
      const rule = styleRules[j];

      // this is necessary to get rid of typescript warning for rule.selectorText
      if (!(rule instanceof CSSStyleRule)) continue;

      if (rule.selectorText === `.${node.className}`) {
        const initialText = rule.cssText; // getting the full text of the CSS rule
        const i = initialText.indexOf('{'); // finding index of first bracket
        innerText = initialText.substring(i + 1, initialText.length - 1); // extracting the CSS to insert into inline style
        // Adjusting the width and height if they are set via the custom class
        const arr = innerText.split(' ');
        arr.forEach((str, i) => {
          if (str === 'width:') {
            width = str.concat(arr[i + 1]); // go through the array and join width and the number
            const w = parseInt(arr[i + 1]); // getting the number for the width
            width = w;
          }
          if (str === 'height:') {
            height = str.concat(arr[i + 1]); // same as with the width
            const h = parseInt(arr[i + 1]);
            height = h;
          }
        });
      }
    }
  }

  // update the width/height of the node
  store.nodesStore.update((nodes) => {
    if (width !== undefined) nodes[node.id].width = width;
    if (height !== undefined) nodes[node.id].height = height;
    return { ...nodes };
  });

  // move the node a bit to force update to anchors, potential anchors
  node.setPositionFromMovement(0, 0);

  // update the position of the resize node. TODO: this should be refactored so that it cascades from movement
  store.resizeNodesStore.update((resizeNodes) => {
    for (let resizeNode of Object.values(resizeNodes)) {
      if (resizeNode.nodeId === node.id) {
        // set resizeNode position to bottom left corner of Node
        resizeNode.positionX = node.positionX + node.width;
        resizeNode.positionY = node.positionY + node.height;
      }
    }
    return { ...resizeNodes };
  });
};
