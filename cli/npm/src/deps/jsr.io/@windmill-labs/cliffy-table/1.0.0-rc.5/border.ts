/** Default border characters. */
export const border = {
  top: "─",
  topMid: "┬",
  topLeft: "┌",
  topRight: "┐",
  bottom: "─",
  bottomMid: "┴",
  bottomLeft: "└",
  bottomRight: "┘",
  left: "│",
  leftMid: "├",
  mid: "─",
  midMid: "┼",
  right: "│",
  rightMid: "┤",
  middle: "│",
};

/** Default border characters. */
export type Border = typeof border;

/** @deprecated Use `Border` instead. */
export type IBorder = Border;
