import DiffMatchPatch from "diff-match-patch";

const dmp = new DiffMatchPatch();

export const computeDiffRanges = (
  original: string,
  polished: string,
): Array<{ start: number; end: number }> => {
  if (original === polished) return [];

  const diffs = dmp.diff_main(original, polished);
  dmp.diff_cleanupSemantic(diffs);

  const ranges: Array<{ start: number; end: number }> = [];
  let position = 0;

  for (const [op, text] of diffs) {
    if (op === DiffMatchPatch.DIFF_INSERT) {
      ranges.push({ start: position, end: position + text.length });
      position += text.length;
    } else if (op === DiffMatchPatch.DIFF_EQUAL) {
      position += text.length;
    }
    // DIFF_DELETE: text was removed, doesn't affect position in polished string
  }

  return ranges;
};
