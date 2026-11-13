export const ROLE_RULES = [
  ["character", /^(character-|hero|keeper|knight)/],
  ["prop", /^(stump|trunk|tree-trunk|tree-autumn-trunk|tree-log)/],
  ["tree", /^(tree|plant|pine|tall-tree|tree_skinned|dead-tree)/],
  ["nature", /^(grass|patch-|hedge|bush|flower|mushroom|ground-hills)/],
  ["rock", /^(rock|rocks|stones?$|stones-|resource-|gate-rock|debris-|gravestone-debris)/],
  ["effect", /^(indicator|arrow|blade|cone$)/],
  ["item", /^(aid|bottle|box|bucket|coin|key|chest|fish|food|bread|candle|book|cog|lever|button|coffin)/],
  ["prop", /^(lantern|lightpost|light|campfire|fire-|traffic|electricity|construction|crane|banner|flag|stall|cart|barrel|bedroll|bench|chair|dumpster|hay|ladder|conveyor|hopper|machine|chimney|catwalk|oopi|fence|iron-fence|metal-|overhang|door|gate|cross|torch|post-|brazier|standard|tombstone)/],
  ["building", /^(tower|building|wall|structure|template|brick-wall|balcony|crypt|stairs|siege|bridge|column|pillar|altar|water-tower|corridor|shop|fountain|tent|road|grave|border-pillar|broken-column|blocks-ruined)/],
];

export function roleOf(stem) {
  for (const [role, re] of ROLE_RULES) if (re.test(stem)) return role;
  return "decoration";
}

export function kindOf(stem) {
  return stem.replace(/-/g, "_");
}
