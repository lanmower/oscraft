# RPG ability definitions. effect() returns a handle that ability() consumes.
# Ability indices follow declaration order: strike 0, mend 1, bite 2.
ability("strike", 8, 0, "unit", [effect("damage", p0=8)], target_filters=["ground"])
ability("mend", 30, 20, "none", [effect("heal", p0=10, p1=3000)])
ability("bite", 6, 0, "unit", [effect("damage", p0=5)], target_filters=["ground"])
