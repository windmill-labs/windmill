export function main(element: number) {
  const sequence = [0, 1];
  for (let i = 2; i <= element; i++) {
    sequence[i] = sequence[i - 2] + sequence[i - 1];
  }
  return sequence[element];
}
