-- The moba pack's root. It names the features in hook order and nothing else; each feature returns its own hooks.
require("aegis") -- first: a death it holds is read as unrewarded by the death hooks that follow
require("heroes")
require("clock")
require("camps")
require("lanes")
require("store")
require("talents")
require("skills")
require("spells")
require("protection")
require("win")
require("effects")
require("controls")
require("panels")
require("kill_gold")
require("taunt")
require("levelup")
require("upgrades")
require("targeting")
require("unit_rules") -- last: it sets the next tick's armour, speed bounds and miss chances from this tick's state
return {}
