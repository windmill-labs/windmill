export function isLikelyMutatingBashCommand(command: string): boolean {
  return (
    /\b(?:mkdir|touch|rm|mv|cp|install|tee)\b/.test(command) ||
    /\b(?:cat|echo|printf)\b.*(?:>|>>|\|\s*tee\b)/.test(command) ||
    /\bsed\s+-i\b/.test(command) ||
    /\bperl\s+-pi\b/.test(command) ||
    // In command position only: `cat AGENTS.wmill.md` is a read.
    /(?:^|[;&|(]|\b(?:then|do|xargs)\s)\s*wmill(?:\s|$)/m.test(command)
  );
}
