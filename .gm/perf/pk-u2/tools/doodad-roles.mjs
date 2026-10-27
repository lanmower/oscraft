// Role of each shipped GLB model (scenarios/moba/assets/models/<stem>.glb). Rules are checked in order; the first match wins,
// and a model matching none is a decoration. The kind name of a model is its stem with "-" turned into "_".
export const ROLE_RULES = [
  ["character", /^(character-|hero|keeper|knight)/],
  ["prop", /^(stump|trunk|tree-trunk|tree-autumn-trunk|tree-log)/], // cut trunks and logs are stumps, sized as props, not as trees
  // "tall-tree" must match: it is the model the map places for the plain "tree" kind, and missing it
  // made it fall through to decoration, which sized the 3475 placed trees as if they were trinkets.
  ["tree", /^(tree|plant|pine|tall-tree|tree_skinned|dead-tree)/],
  ["nature", /^(grass|patch-|hedge|bush|flower|mushroom|ground-hills)/],
  ["rock", /^(rock|rocks|stones?$|stones-|resource-|gate-rock|debris-|gravestone-debris)/],
  ["effect", /^(indicator|arrow|blade|cone$)/],
  ["item", /^(aid|bottle|box|bucket|coin|key|chest|fish|food|bread|candle|book|cog|lever|button|coffin)/],
  // Torches, posts, braziers, standards and tombstones are wall or ground props at waist height. They used to fall through
  // to decoration, whose 0.35-cell floor drew them as trinkets (the torches were 54 placements at that floor).
  ["prop", /^(lantern|lightpost|light|campfire|fire-|traffic|electricity|construction|crane|banner|flag|stall|cart|barrel|bedroll|bench|chair|dumpster|hay|ladder|conveyor|hopper|machine|chimney|catwalk|oopi|fence|iron-fence|metal-|overhang|door|gate|cross|torch|post-|brazier|standard|tombstone)/],
  // Broken columns and ruined blocks are structures, not debris: "broken" and "ruined" fell through to decoration.
  ["building", /^(tower|building|wall|structure|template|brick-wall|balcony|crypt|stairs|siege|bridge|column|pillar|altar|water-tower|corridor|shop|fountain|tent|road|grave|border-pillar|broken-column|blocks-ruined)/],
];

export function roleOf(stem) {
  for (const [role, re] of ROLE_RULES) if (re.test(stem)) return role;
  return "decoration";
}

export function kindOf(stem) {
  return stem.replace(/-/g, "_");
}
