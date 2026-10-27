local text = {}

text.names = {
  [0] = "Player's names restored to default",
  [1] = "Player's names turned into numbers",
  [2] = "Player's names turned into hero names",
}

text.noHeroName = "No Hero"
text.noHeroSelected = "No Hero is selected"
text.movespeed = "movespeed is"
text.controls = "controls"
text.level = "Level"
text.left = "Left"
text.type = "Type"
text.creepsKilled = "Creeps Killed:"
text.creepsDenied = "Creeps Denied:"
text.neutralsKilled = "Neutrals Killed:"
text.doubleHold = { [false] = "Double Hold feature turned |c00FF0000OFF|r", [true] = "Double Hold feature turned |c0000FF00ON|r" }
text.denyInfo = "-di functionality is on by default. Use -cson and -csoff to toggle it on and off."
text.spam = "Do not spam."
text.randomSoon = "You will be given a random hero soon."
text.randomRefused = "You cannot be assigned a random hero."
text.rolled = "You randomly rolled"
text.shared = "|c006699CC Shared unit control with "
text.unshared = "|c006699CC Took unit control away from "
text.hostOnly = "Only the host (Player 1) can kick players!"
text.kickRunning = "There is already a kickrequest running. You need to wait till it has ended."
text.kickChoose = "You have to choose a valid player: "
text.kickRequested = " has requested to kick player: "
text.kickAccept = " Type |c006699CC-yes|r to accept."
text.kickAccepted = " accepted the request. "
text.kickExpired = "|c00ff0303Kick time has expired without enough votes."
text.kickRemoved = " has been removed from the game."
text.afkFor = "has been afk for more than"
text.afkMinutes = "minutes"
text.afkType = "You may type"
text.afkBoot = "to boot this player from the game"
text.afkBooted = "has been booted for being afk too long!"
text.musicHelp = "Command is used by typing -music theme. Where theme is one of the following:"
text.switchOff = "-switch has been disabled for this game."
text.switchLocked = "The enemy team has unlocked that hero's items. He cannot be switched."
text.switchTo = "to switch with"
text.switchRequest = "$c1Type|r $c2-switch accept|r $c1or|r $c2-ok|r $c1to allow|r $p1 $c1to switch with|r $p2"
text.switchMine = "$c1You have requested to switch places with|r $p2"
text.switchVeto = "$c1If you want veto this, type |r$c2-no|r"
text.switchRejected = "$p1 $c1has rejected the switch offer."
text.switchExpired = "Switch time has expired without enough votes"
text.switchAccepted = "has accepted the switch offer"
text.switchDone = "$c1The switch process between|r $p1 $c1and|r $p2 $c1has been completed successfully|r"
text.pickAgain = "You can now pick a new hero"
text.repickLater = "is disabled for a few seconds"
text.repickOff = "Repicking has been disabled by the host"
text.repickLate = "Too late to repick"
text.repickGold = "Not enough gold to repick"
text.repickSkin = "You cannot repick after changing your skin."
text.swapOff = "has been disabled."
text.swapLate = "Too late to use"
text.swapNone = "No hero available to swap with."
text.swapNoHero = "You must have a living hero in order to swap with someone."
text.swapInvalid = "Invalid swap selection"
text.swapOptions = "Swap Hero Options:"
text.swapRequested = "Requested Swap With You"
text.swapChoose = "to make a choice or"
text.swapCancel = "to cancel swap requests"
text.swapWants = "wants to swap with you."
text.swapAccept = "to accept"
text.swapYouRequested = "You have requested to swap with"
text.swapSkin = "You cannot swap after changing your skin."

text.musicTracks = {
  "Sound\\Music\\mp3Music\\NightElf1.mp3", "Sound\\Music\\mp3Music\\NightElf2.mp3", "Sound\\Music\\mp3Music\\NightElf3.mp3",
  "Sound\\Music\\mp3Music\\Human1.mp3", "Sound\\Music\\mp3Music\\Human2.mp3", "Sound\\Music\\mp3Music\\Human3.mp3",
  "Sound\\Music\\mp3Music\\Orc1.mp3", "Sound\\Music\\mp3Music\\Orc2.mp3", "Sound\\Music\\mp3Music\\Orc3.mp3",
  "Sound\\Music\\mp3Music\\Undead1.mp3", "Sound\\Music\\mp3Music\\Undead2.mp3", "Sound\\Music\\mp3Music\\Undead3.mp3",
  "Sound\\Music\\mp3Music\\BloodElfTheme.mp3", "Sound\\Music\\mp3Music\\DarkAgents.mp3", "Sound\\Music\\mp3Music\\TragicConfrontation.mp3",
  "Sound\\Music\\mp3Music\\SadMystery.mp3", "Sound\\Music\\mp3Music\\IllidansTheme.mp3", "Sound\\Music\\mp3Music\\Tension.mp3",
  "Sound\\Music\\mp3Music\\ArthasTheme.mp3", "Sound\\Music\\mp3Music\\NightElfDefeat.mp3", "Sound\\Music\\mp3Music\\NightElfVictory.mp3",
  "Sound\\Music\\mp3Music\\PH1.mp3",
}

text.musicNames = {
  "nightelf1", "nightelf2", "nightelf3", "human1", "human2", "human3", "orc1", "orc2", "orc3", "undead1", "undead2", "undead3",
  "other1", "other2", "other3", "other4", "other5", "other6", "other7", "other8", "other9", "special",
}

text.musicList = "off, random, nightelf1, nightelf2, nightelf3, human1, human2, human3, orc1, orc2, orc3, undead1, undead2, undead3, other1, other2, other3, other4, other5, other6, other7, other8, other9, special"

text.camera = {
  wrongMultiplier = "Incorrect multiplier, camera changed to default.",
  wrongDistance = "Incorrect camera distance.",
}

text.weather = { off = 0, snow = 1, rain = 2, moonlight = 3, wind = 4, random = 5 }

text.gameInfo = {
  { "lm", { "|c006699CCLeague Mode|r:\t\t\tBoth Sides will take turns picking heroes." } },
  { "xl", { "|c006699CCExtended League|r:\t\t\tBoth Sides will take turns picking heroes.", "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\tEach team captain can pick 4 heroes to remove from the pool." } },
  { "dm", { "|c006699CCDeath Match|r:\t\t\t\tWhen you die, you must pick a new hero." } },
  { "mm", { "|c006699CCMirror Match|r:\t\t\tAfter one minute, each team will have the same heroes." } },
  { "ap", { "|c006699CCAll Pick|r:\t\t\t\t\t\t\t\tYou may pick a hero from any tavern." } },
  { "rd", { "|c006699CCRandom Draft|r:\t\t\t\t\t\t\t\tYou may pick a hero draft style from the heroes available." } },
  { "sd", { "|c006699CCSingle Draft|r:\t\t\t\t\t\t\t\tYou may pick a hero from one of the three options." } },
  { "ar", { "|c006699CCAll Random|r:\t\t\t\t\tYou will be given a random hero from any tavern." } },
  { "vr", { "|c006699CCVote Random|r:\t\t\t\tYou will be given a random hero from the voted groupset." } },
  { "tr", { "|c006699CCTeam Random|r:\t\tYou will be given a random hero from your taverns." } },
  { "rv", { "|c006699CCReverse|r:\t\t\t\t\t\t\t\tYou pick a hero for your opponent." } },
  { "sp", { "|c006699CCShuffle Players|r:\t\tAll players will be shuffled across both sides." } },
  { "aa", { "|c006699CCAll Agility|r:\t\t\t\t\t\tOnly Agility heroes will be allowed." } },
  { "as", { "|c006699CCAll Strength|r:\t\t\t\tOnly Strength heroes will be allowed." } },
  { "ai", { "|c006699CCAll Intelligence|r:\t\tOnly Intelligence heroes will be allowed." } },
  { "ro", { "|c006699CCRange Only|r:\t\tOnly range heroes are available" } },
  { "mo", { "|c006699CCMelee Only|r:\t\tOnly melee heroes are available" } },
  { "du", { "|c006699CCDuplicate Mode|r:\tThe same heroes may be picked or randomed multiple times." } },
  { "id", { "|c006699CCItem Drop|r:\t\t\t\t\t\tWhen you die, a random inventory slot will drop an item." } },
  { "np", { "|c006699CCNo Powerups|r:\t\t\t\tPowerups are disabled." } },
  { "sc", { "|c006699CCSuper Creeps|r:\t\t\t\tSiege Golems, Scary Fish, and Ancient Hydras", "\t\t\t\t\t\t\t\t\t\t\t\t\t\tcan spawn randomly with creep spawns." } },
  { "em", { "|c006699CCEasy Mode|r:\t\t\t\t\tAll players get more gold and XP. Towers are weaker." } },
  { "sh", { "|c006699CCSame Hero|r:\t\t\t\t\tAll players will be given the same hero that %s|r has." } },
  { "om", { "|c006699CCOnly Mid|r:\t\t\t\t\tOnly middle towers can be destroyed" } },
  { "nb", { "|c006699CCNo Bot|r:\t\t\t\t\tCreeps won't spawn from the bottom lane" } },
  { "nm", { "|c006699CCNo Mid|r:\t\t\t\t\tCreeps won't spawn from the middle lane" } },
  { "nt", { "|c006699CCNo Top|r:\t\t\t\t\tCreeps won't spawn from the top lane" } },
  { "ns", { "|c006699CCNo Swap|r:\t\t\t\t\tSwapping heroes is not allowed" } },
  { "nr", { "|c006699CCNo Repick|r:\t\t\t\t\tRepicking heroes is not allowed" } },
  { "pm", { "|c006699CCPool Mode|r:\t\t\t\t\tAllows allies to pool each other items" } },
  { "oi", { "|c006699CCObserver Info|r:\t\t\t\t\tDisables extra information for Observers" } },
  { "mi", { "|c006699CCMini Hereos|r:\t\t\t\t\tCreates minature models of the heroes" } },
  { "fr", { "|c006699CCFast Respawn|r:\t\t\t\t\tReduces death times by 50%" } },
  { "cm", { "|c006699CCCaptains Mode|r:\t\t\t\t\tA captain from each side will ban and pick heroes." } },
  { "er", { "|c006699CCExperimental Runes|r:\t\t\t\t\tA different system for spawning runes" } },
  { "cd", { "|c006699CCCaptains Draft|r:\t\t\t\t\tCaptains draft heroes from a limited pool" } },
  { "cp", { "|c006699CCCapture Point|r:\t\t\t\t\tCapture Points will appear at the 10 minute mark. The more heroes the faster the capture rate." } },
  { "zm", { "|c006699CCZoom Mode|r:\t\t\t\t\tObservers will view the game from a zoomed out state." } },
  { "normal", { "|c006699CCNormal Mode|r:\t\t\t\tNo mode has been specified" } },
}

return text
