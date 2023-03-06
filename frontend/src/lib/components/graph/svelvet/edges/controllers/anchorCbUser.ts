/**
 *  These are callbacks used to define anchor positions relative to the node they are attached to.
 *  These may be provided to developers as examples of how to write their own custom callbacks for adjustable anchors
 *  It calculates the position of an anchor (x,y) coordinates given a node parameterized by (x,y,width, height)
 */

/**
 * @function rightCb - This is a callback function to define the anchor position on the node to be on the right side of the node.
 * @param xNode - positionX of the attached node
 * @param yNode - positionY of the attached node
 * @param widthNode -  width of the attached node
 * @param heightNode - height of the attached node
 * @returns [xAnchor, yAnchor, 0]
 * xAnchor - positionX for the anchor
 * yAnchor - positionY for the anchor
 * 0 - this is angle of the anchor with respect to the node. The right anchor should fall at 0 degrees on the unit circle.
 * @export rightCb
 */
export const rightCb = (
  xNode: number,
  yNode: number,
  widthNode: number,
  heightNode: number
) => {
  const xAnchor = xNode + widthNode;
  const yAnchor = yNode + heightNode / 2;
  return [xAnchor, yAnchor, 0];
};

/**
 * @function leftCb - This is a callback function to define the anchor position on the node to be on the left side of the node.
 * @param xNode - positionX of the attached node
 * @param yNode - positionY of the attached node
 * @param widthNode - width of the attached node
 * @param heightNode - height of the attached node
 * @returns [xAnchor, yAnchor, 180]
 *   xAnchor - positionX for the anchor
 *   yAnchor - positionY for the anchor
 *   180 - this is angle of the anchor with respect to the node. The left anchor should fall at 180 degrees on the unit circle.
 * @export leftCb
 */
export const leftCb = (
  xNode: number,
  yNode: number,
  widthNode: number,
  heightNode: number
) => {
  const xAnchor = xNode;
  const yAnchor = yNode + heightNode / 2;
  return [xAnchor, yAnchor, 180];
};

/**
 * @function topCb - This is a callback function to define the anchor position on the node to be on the top of the node.
 * @param xNode - positionX of the attached node
 * @param yNode - positionY of the attached node
 * @param widthNode - width of the attached node
 * @param heightNode - height of the attached node
 * @returns [xAnchor, yAnchor, 90]
 *   xAnchor - positionX for the anchor
 *   yAnchor - positionY for the anchor
 *   90 - this is angle of the anchor with respect to the node. The top anchor should fall at 90 degrees on the unit circle.
 * @export topCb
 */

export const topCb = (
  xNode: number,
  yNode: number,
  widthNode: number,
  heightNode: number
) => {
  const xAnchor = xNode + widthNode / 2;
  const yAnchor = yNode;
  return [xAnchor, yAnchor, 90];
};

/**
 * @function bottomCb - This is a callback function to define the anchor position on the node to be on the bottom of the node.
 * @param xNode - positionX of the attached node
 * @param yNode - positionY of the attached node
 * @param widthNode - width of the attached node
 * @param heightNode - height of the attached node
 * @returns - [xAnchor, yAnchor, 90]
 *   xAnchor - positionX for the anchor
 *   yAnchor - positionY for the anchor
 *   270 - this is angle of the anchor with respect to the node. The bottom anchor should fall at 270 degrees on the unit circle.
 * @export bottomCb
 */
export const bottomCb = (
  xNode: number,
  yNode: number,
  widthNode: number,
  heightNode: number
) => {
  const xAnchor = xNode + widthNode / 2;
  const yAnchor = yNode + heightNode;
  return [xAnchor, yAnchor, 270];
};
