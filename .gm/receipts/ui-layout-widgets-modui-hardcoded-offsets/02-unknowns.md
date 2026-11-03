# 02-unknowns: ui-layout-widgets-modui-hardcoded-offsets
session: gm-drain-1793526214-17

mutables added (mutable-add, row ui-layout-widgets-modui-hardcoded-offsets):
1. ui-offsets-fallbacks-are-only-px-left: are the var(--bar-h,160px)/(--bar-h,120px) fallbacks (widgets.js:107,108,115; modui.js:8,15,16) the only fixed bar-height pixels left?
2. ui-offsets-bar-h-defined-everywhere-widgets-renders: is --bar-h set on :root unconditionally in play.html for every page loading these modules?
3. ui-offsets-844x390-map-visible-above-half: measured at 844x390, is the map above 50 percent after panels lay out?
4. ui-offsets-844x390-widgets-clamp-missing: do #ui-left/#ui-right overflow at 844x390 since the clamp sits under max-width:600px (widgets.js:106)?
5. ui-offsets-other-importers-of-widgets-modui: any importer besides play.html's graph?