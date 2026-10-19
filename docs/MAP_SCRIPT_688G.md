# DotA Allstars 6.88g war3map.j: function index and literal facts

Source: `war3map.j` extracted from `DotA v6.88g Allstars.w3x` (3,065,090 bytes, 102,278 lines). The text is obfuscated (random identifiers, locals reused for unrelated values), so behaviour below is read from bodies, not names. No JASS interpreter was run. Line numbers are 1-based in `war3map.j`.

Method: a Node text scan finds each `function ... endfunction` pair (4,503 functions) and counts the natives each body calls. A function is tagged with a direct effect when its body calls a unit, timer, stock, time, hero or tech native. Tags are counts of native use, not proof of intent. Hand-read rows are marked as such.

## 1. Function index

- Functions: 4,503. With a direct effect: 1,086 (table in section 1.3). Helpers without a direct native effect: 3,417 (forward, compute or store values).
- Tags: `units` (CreateUnit, CreateItem, UnitAddItemById, AddUnitToStock, CreateDestructable), `timers` (TimerStart, TriggerRegisterTimerEvent*), `stock` (SetPlayerState), `time` (time-of-day natives), `hero` (SetHeroLevel, AddHeroXP), `tech` (SetPlayerTechResearched*).

### 1.1 Hand-read clock and wave functions

| Function | Lines | Behaviour (read from body) |
| --- | --- | --- |
| `main` | 99882-102203 | Map init. Sets time of day 6.0 (100916), time-of-day scale .5 (100918), creep camp filter on (100920), play rect (100915), weather effects, player arrays `ne[]`/`Ve[]`, and registers the wave trigger handler. |
| `PRE` | 30017-30112 | Game-start clock handler, run by the `SA` countdown timer. At zero: starts the game (30028-30031), sets time of day 6.0 (30033) and scale .625 (30035), starts a one-shot 180 s timer `POE` (30036), and registers the periodic and one-shot clock triggers (30045-30068, listed in section 2.2). From 30070 on it posts countdown text and deducts 1 gold from each empty slot every trigger cycle (30096-30108). |
| `JAE` | 26200-26423 | 30 s wave handler (`xR`, registered at 101660). Every run spawns the lane units (26227-26238, and the per-lane branches to 26296). Run 1 resets cooldowns (26210-26221) and adds 3 `n137` to two stocks (26222-26225). On every 7th run (`wT`, 26201) a second block (26297 on) spawns `umtw` and `u00R` for the lanes. The identity of those two codes is not established in the map text. |
| `HSE` | 26000-26002 | One-shot 2669 s trigger handler (`vR`, 30053). Only increments counter `Ji`. |
| `HTE` | 26003-26005 | One-shot 849 s, 1759 s and 2669 s handler (`eR`, 30054-30056). Only increments counter `ji`. |
| `JTE` | 26536-26545 | 450 s periodic handler (`oR`, 30052, registered at 101704). Calls `SetPlayerTechResearchedSwap` once per tech: `R007`, `R00C`, `R008`, `R00D` for player `ne[0]` and `R009`, `R003`, `R00A`, `R00B` for player `Ve[0]`, each count plus 1. |
| `G4V` | 14792-14800 | 240 s periodic handler (`zD`, 30057). If a unit is alive, sets tech `R00F` for its owner and calls `G0V` and `G1V`. |
| `KPE` | 26790-26813 | 60 s condition handler (`iR`, 30045). Calls `KKE` five times on groups (26796-26800 on). `KKE` body not read. |
| `KQE` | 26814-26938 | 600 s periodic action (`aR`, 30061-30063, only if `Xi`). Picks `GetRandomInt(1,9)` and spawns at rect centres. |
| `USX` | 51460-51484 | Day/night helper. Suspends time of day for `Ju` seconds, optionally sets time of day to 0 when `b` is true (51480). |
| `SVE` | 31155-31185 | Sets hero level 10 (`0xA`) at 31172 and 31176. Caller and condition not read. |
| `BMJ` | 99409-99707 | Save-string decoder. Reads time (`SetTimeOfDay(S2R(SubString(ASJ,6,9)))`, 99552), lumber (99427) and hero level (99643) from a string. Values are runtime, not literal. |

### 1.2 Timers armed in PRE (30017-30112)

| Line | Timer | Interval | Handler | Notes |
| --- | --- | --- | --- | --- |
| 30036 | one-shot | 180 s (`0xB4`) | `POE` | - |
| 30045 | periodic | 60 s | `iR` (`KPE`) | - |
| 30047 | one-shot | 30 s | `Nu` (`PXE`) | - |
| 30050 | periodic | 30 s | `xR` (`JAE`) | Also run once at 30051 (`TriggerExecute`), so run 1 is at game start. |
| 30052 | periodic | 450 s | `oR` (`JTE`) | - |
| 30053 | one-shot | 2669 s (`2699-30`) | `vR` (`HSE`) | - |
| 30054 | one-shot | 849 s (`999-150`) | `eR` (`HTE`) | - |
| 30055 | one-shot | 1759 s (`1999-240`) | `eR` (`HTE`) | - |
| 30056 | one-shot | 2669 s (`2999-330`) | `eR` (`HTE`) | - |
| 30057 | periodic | 240 s (`60*4`) | `zD` (`G4V`) | - |
| 30058 | periodic | 30 s | `ib` (`T2`) | - |
| 30061-30063 | periodic | 600 s | `aR` (`KQE`) | Only if `Xi`. |
| 30068 | periodic | 30 s | `Nu` (`JXE`) | Armed when `fT==7`. |

Other 30 s periodic timers in the script: none literal in the 1.2 list beyond the above.

### 1.3 Functions with a direct effect (1,086)

Columns: function, lines, tags. Generated from the scan, not hand-read. Use section 1.1 for the hand-read ones.

| Function | Lines | Tags |
| --- | --- | --- |
| yS | 2347-2378 | timers |
| nt | 2391-2399 | timers |
| It | 2420-2425 | units |
| bt | 2440-2451 | timers |
| Ft | 2502-2508 | units |
| Qt | 2581-2586 | timers |
| Zt | 2670-2685 | timers |
| vT | 2686-2704 | units |
| eT | 2705-2720 | timers |
| oT | 2729-2735 | timers |
| aT | 2744-2750 | timers |
| LT | 2933-2936 | timers |
| bu | 3198-3206 | timers |
| OLV | 3234-3242 | timers |
| Gu | 3251-3259 | timers |
| Ku | 3268-3276 | timers |
| CU | 3681-3703 | stock |
| jU | 3805-3841 | units |
| JU | 3842-3878 | units |
| kU | 3879-3915 | units |
| KU | 3916-3952 | units |
| tw | 4235-4279 | units |
| Uw | 4295-4305 | timers |
| VW | 4434-4440 | units |
| XW | 4451-4458 | timers |
| NW | 4485-4492 | timers |
| qW | 4622-4631 | timers |
| tW | 4641-4651 | timers |
| uW | 4661-4670 | timers |
| ey | 4704-4711 | timers |
| Jy | 4817-4822 | units |
| OUV | 4853-4862 | timers |
| Sy | 4887-4893 | timers |
| wy | 4913-4918 | timers |
| OWV | 4984-5007 | units, timers |
| xY | 5008-5028 | units, timers |
| VY | 5046-5062 | timers |
| XY | 5080-5100 | timers |
| RY | 5113-5120 | timers |
| AY | 5135-5144 | timers |
| bY | 5156-5164 | timers |
| CY | 5180-5187 | timers |
| DY | 5197-5204 | timers |
| hY | 5205-5230 | stock |
| JY | 5242-5249 | timers |
| sY | 5325-5334 | timers |
| SY | 5335-5359 | stock |
| uY | 5373-5392 | timers |
| UY | 5393-5404 | timers |
| yY | 5416-5433 | timers |
| ZY | 5449-5471 | timers |
| vz | 5472-5484 | timers |
| rz | 5524-5555 | timers |
| Vz | 5576-5593 | timers |
| OZV | 5619-5658 | units |
| bz | 5659-5696 | units |
| cz | 5697-5732 | units |
| gz | 5744-5751 | timers |
| Hz | 5768-5776 | timers |
| Wz | 5868-5876 | stock |
| tZ | 6088-6111 | units |
| wZ | 6135-6154 | units, timers |
| x0 | 6219-6254 | timers |
| G0 | 6811-6903 | tech |
| l0 | 6926-7022 | tech |
| O1 | 7127-7154 | units |
| R1 | 7155-7167 | timers |
| f1 | 7236-7275 | units |
| T1 | 7638-7660 | timers |
| z1 | 7678-7698 | units |
| M2 | 8003-8024 | units, timers |
| Q2 | 8049-8067 | units |
| S2 | 8068-8089 | units |
| U2 | 8106-8123 | units |
| w2 | 8124-8147 | units, timers |
| e3 | 8215-8222 | timers |
| d3 | 8439-8479 | timers |
| L3 | 8726-8934 | units, timers, stock |
| d4 | 10053-10089 | timers |
| J4 | 10285-10329 | stock |
| k4 | 10330-10337 | timers |
| y4 | 10500-10648 | timers |
| X5 | 10721-10771 | timers |
| B5 | 10772-10840 | timers |
| d5 | 10841-11321 | units |
| BTV | 11389-11606 | units, timers, stock |
| B9V | 11630-11648 | units, timers |
| CXV | 11655-11675 | stock |
| CNV | 11717-11744 | timers |
| C2V | 13571-13588 | stock |
| DXV | 13690-13696 | hero |
| DDV | 13776-13783 | timers |
| DGV | 13804-13812 | timers |
| DJV | 13841-13844 | stock |
| D1V | 14027-14031 | hero |
| D9V | 14086-14227 | hero |
| FWV | 14228-14237 | timers |
| F_V | 14264-14642 | units, timers |
| GYV | 14721-14730 | timers |
| GZV | 14731-14762 | units |
| G_V | 14763-14771 | timers |
| G4V | 14792-14800 | tech |
| G8V | 14833-14875 | timers |
| HXV | 14894-14906 | units |
| HOV | 14907-14931 | tech |
| HIV | 14938-14976 | units, timers |
| HDV | 15027-15039 | timers |
| HMV | 15117-15138 | stock |
| HPV | 15139-15213 | timers |
| H_V | 15262-15282 | timers |
| H0V | 15283-15369 | units |
| H7V | 15426-15473 | units |
| JXV | 15513-15558 | timers |
| JRV | 15559-15595 | timers |
| JBV | 15643-15780 | units |
| JLV | 15824-15851 | timers |
| JPV | 15852-15878 | timers |
| JTV | 15914-15970 | timers |
| J_V | 15984-16022 | stock |
| J2V | 16048-16089 | units |
| LVV | 16390-16475 | units, stock |
| LLV | 16597-16615 | units |
| LPV | 16619-16639 | stock |
| L2V | 16894-16954 | stock |
| L5V | 17007-17099 | units |
| L6V | 17100-17211 | units |
| L9V | 17218-17288 | units |
| MRV | 17299-17304 | units |
| MJV | 17443-17763 | units |
| PCV | 17830-17845 | stock |
| PFV | 17846-17858 | stock |
| PHV | 17862-17880 | timers |
| PKV | 17895-17900 | timers |
| PLV | 17901-17926 | units |
| PUV | 17927-18122 | units, timers |
| P2V | 18123-18301 | timers |
| QEV | 18368-18379 | timers |
| QOV | 18399-18409 | timers |
| QRV | 18410-18424 | units |
| QIV | 18425-18449 | hero |
| QCV | 18450-18485 | units |
| QDV | 18486-18496 | timers |
| QFV | 18497-18504 | units |
| QKV | 18535-18626 | units, timers |
| QSV | 18676-18693 | timers |
| Q0V | 18786-18790 | units |
| SVV | 18999-19007 | timers |
| SOV | 19064-19085 | units |
| SNV | 19130-19281 | units |
| SPV | 19316-19331 | timers |
| STV | 19344-19351 | timers |
| S0V | 19439-19451 | units |
| S7V | 19543-19563 | timers |
| TVV | 19587-19607 | timers |
| TCV | 19820-19829 | timers |
| THV | 19915-19935 | timers |
| TKV | 19942-19947 | units |
| TMV | 19965-19993 | units |
| TPV | 19994-20003 | timers |
| TWV | 20060-20070 | timers |
| T1V | 20117-20127 | timers |
| T3V | 20147-20155 | units |
| T4V | 20156-20187 | units |
| T5V | 20188-20226 | units |
| T6V | 20227-20248 | timers |
| T7V | 20249-20260 | timers |
| UEV | 20326-20350 | timers |
| UIV | 20385-20412 | units, timers |
| UBV | 20432-20489 | units |
| BYV | 20552-20585 | units |
| UKV | 20586-20603 | units |
| ULV | 20604-20713 | units |
| U1V | 20714-20912 | timers |
| U7V | 20913-20920 | timers |
| U8V | 20921-21033 | units |
| U9V | 21034-21185 | timers |
| WRV | 21212-21221 | timers |
| WAV | 21277-21313 | timers |
| WCV | 21386-21409 | timers |
| WDV | 21410-21427 | units |
| W9V | 21795-21805 | timers |
| YVV | 21806-21912 | units, timers |
| YXV | 21913-22002 | units, timers |
| YRV | 22011-22048 | units |
| YMV | 22150-22174 | units, timers |
| YTV | 22209-22219 | units |
| YUV | 22220-22235 | units |
| YZV | 22267-22287 | timers |
| Y2V | 22316-22383 | timers |
| Y4V | 22384-22400 | timers |
| ZVV | 22465-22475 | units |
| ZIV | 22502-22511 | units |
| ZAV | 22512-22544 | units |
| ZBV | 22551-22568 | timers |
| ZFV | 22581-22586 | timers |
| ZKV | 22643-22667 | units, timers |
| Z0V | 22771-22790 | timers |
| Z6V | 22918-22930 | timers |
| VVE | 22961-22972 | timers |
| VIE | 23020-23037 | units |
| VCE | 23061-23074 | timers |
| VFE | 23098-23131 | timers |
| VHE | 23156-23185 | timers |
| VKE | 23186-23235 | tech |
| VME | 23249-23271 | timers |
| VPE | 23272-23317 | timers |
| VUE | 23346-23359 | timers |
| VWE | 23360-23389 | timers |
| V0E | 23435-23458 | timers |
| EVE | 23511-23536 | timers |
| SoulRing_SpellEffect2 | 23558-23583 | timers |
| EFE | 23681-23720 | timers |
| EJE | 23735-23786 | units, timers |
| EPE | 23787-23837 | units, timers |
| E2E | 23913-23981 | timers |
| E3E | 23982-24007 | timers |
| E8E | 24049-24069 | units, timers |
| Rtv | 24077-24086 | units |
| XOE | 24103-24105 | timers |
| XAE | 24152-24163 | timers |
| XBE | 24173-24246 | units |
| XLE | 24274-24283 | timers |
| XME | 24284-24324 | units |
| XQE | 24325-24349 | timers |
| XWE | 24372-24384 | units |
| XZE | 24402-24434 | units, timers |
| X3E | 24480-24491 | units |
| X7E | 24508-24538 | hero |
| OXE | 24582-24596 | timers |
| ORE | 24610-24623 | timers |
| OFE | 24691-24720 | timers |
| OHE | 24738-24755 | timers |
| OJE | 24756-24829 | tech |
| OQE | 24855-24903 | stock |
| OWE | 24953-24969 | timers |
| O2E | 25034-25069 | units, timers |
| O7E | 25098-25105 | timers |
| O8E | 25111-25121 | units |
| HRE | 25684-25694 | hero |
| REE | 25695-25700 | units |
| HCE | 25791-25812 | units, timers |
| HFE | 25834-25856 | timers |
| HKE | 25942-25947 | timers |
| HLE | 25948-25964 | timers |
| JRE | 26191-26199 | units |
| JAE | 26200-26423 | units |
| JWE | 26594-26599 | units |
| MCE | 29632-29638 | timers |
| MKE | 29788-29819 | timers |
| PRE | 30017-30112 | timers, stock, time |
| PPE | 30247-30254 | timers |
| PTE | 30312-30319 | timers |
| P2E | 30492-30503 | timers |
| P4E | 30512-30527 | timers |
| QVE | 30564-30624 | timers |
| QIE | 30666-30683 | timers |
| Q_E | 31041-31056 | hero |
| Q0E | 31057-31076 | hero |
| Q3E | 31106-31115 | timers |
| Q6E | 31124-31131 | time |
| Q7E | 31132-31140 | stock |
| Q8E | 31141-31145 | stock |
| Q9E | 31146-31154 | timers |
| SVE | 31155-31185 | units, hero |
| SBE | 31218-31306 | timers |
| SSE | 31380-31403 | timers |
| S2E | 31527-31536 | timers |
| S8E | 31594-31641 | units |
| TQE | 32118-32141 | timers |
| T7E | 32286-32288 | units |
| T9E | 32302-32325 | tech |
| UAE | 32369-32374 | units |
| UGE | 32428-32435 | units |
| UME | 32463-32472 | timers |
| UZE | 32519-32525 | units |
| U0E | 32532-32538 | units |
| WJE | 32812-32835 | units |
| ZOE | 33114-33124 | timers |
| XRX | 33426-33439 | units |
| XCX | 33481-33506 | timers |
| XBX | 33507-33525 | timers |
| BWV | 33537-33561 | units, timers |
| XXX | 33678-33746 | units |
| XQX | 33791-33818 | stock |
| ECX | 33819-33834 | timers |
| XUX | 33897-34001 | units |
| XZX | 34140-34359 | units, stock |
| OOX | 34553-34575 | timers |
| EGX | 34588-34602 | timers |
| OFX | 34869-34877 | timers |
| OHX | 34916-34945 | timers |
| OKX | 35124-35129 | timers |
| OTX | 35214-35232 | timers |
| RMX | 36193-36235 | timers |
| RWX | 36534-36551 | timers |
| RZX | 36632-36645 | units, timers |
| I9X | 37247-37267 | units |
| NXX | 37388-37402 | tech |
| NGX | 37738-37764 | timers |
| NHX | 37765-37796 | units |
| NKX | 37804-37858 | units, timers, stock, time |
| NQX | 37890-37926 | timers, stock |
| NSX | 37927-37966 | timers, stock |
| NTX | 37967-38009 | timers, stock |
| NUX | 38010-38049 | timers, stock |
| NWX | 38050-38092 | timers, stock |
| NYX | 38093-38132 | timers, stock |
| NZX | 38133-38172 | timers, stock |
| N_X | 38173-38212 | timers, stock |
| N0X | 38213-38254 | timers, stock |
| N1X | 38255-38294 | timers, stock |
| N2X | 38295-38334 | timers, stock |
| N3X | 38335-38374 | timers, stock |
| N4X | 38375-38416 | timers, stock |
| VSX | 38417-38456 | timers, stock |
| N5X | 38457-38495 | timers, stock |
| N6X | 38496-38535 | timers, stock |
| N7X | 38536-38578 | timers, stock |
| N8X | 38579-38618 | timers, stock |
| N9X | 38619-38658 | timers, stock |
| BVX | 38659-38698 | timers, stock |
| BEX | 38699-38718 | timers, stock |
| ALX | 38719-38789 | units, timers, stock, time |
| BRX | 38807-38810 | units |
| BCX | 38882-38892 | timers |
| BPX | 38986-38994 | timers |
| BQX | 38995-39024 | units |
| BWX | 39058-39137 | units |
| BYX | 39138-39335 | timers, time |
| B0X | 39372-39394 | timers, time |
| AMX | 39395-39582 | units, timers, stock |
| B1X | 39583-39614 | units |
| B2X | 39615-39666 | units, time |
| B8X | 39797-39800 | units |
| CRX | 39924-39959 | timers, time |
| CAX | 39964-40053 | timers |
| CNX | 40054-40089 | timers |
| A8X | 40090-40318 | units, timers, stock, time |
| APX | 40399-40483 | units |
| CKX | 40484-40541 | units |
| AGX | 40542-40602 | units, timers |
| CPX | 40609-41220 | units, timers |
| AHX | 41288-41405 | timers |
| AKX | 41457-41519 | timers |
| C4X | 41564-41671 | units |
| ARX | 41678-41690 | timers |
| AJX | 41691-41783 | units |
| C9X | 41806-41825 | timers |
| DEX | 41826-41897 | units, stock |
| AVX | 41963-42172 | units |
| AEX | 42173-42339 | units |
| DLX | 42340-42363 | units |
| DMX | 42364-42386 | timers, stock |
| DSX | 42497-42509 | timers |
| A9X | 42652-42667 | timers |
| D8X | 42674-42685 | stock, hero |
| FEX | 42813-42916 | units, timers |
| PIE | 42917-42925 | timers |
| FBX | 43012-43019 | stock |
| FDX | 43023-43057 | units |
| FJX | 43127-43146 | timers |
| FLX | 43179-43288 | units, timers |
| NEX | 43289-43320 | timers |
| F4X | 43386-43419 | units |
| F5X | 43420-43452 | units, timers |
| l_v | 43473-43497 | timers |
| GRX | 43589-43627 | units |
| GIX | 43628-43651 | timers |
| RUE | 43685-43725 | timers |
| GBX | 43726-43749 | timers |
| GCX | 43750-43776 | timers |
| GGX | 43823-43857 | timers |
| F1E | 43898-43908 | timers |
| GUX | 43982-44027 | units, timers |
| G1X | 44089-44117 | timers |
| G2X | 44118-44138 | timers |
| G7X | 44171-44179 | timers |
| RYE | 44335-44393 | units, timers |
| RWE | 44394-44451 | units, timers |
| HBX | 44455-44484 | units |
| HHX | 44510-44524 | units, timers |
| RZE | 44636-44660 | timers |
| HWX | 44679-44713 | timers, stock |
| R_E | 44764-44782 | units, timers |
| H1X | 44794-44803 | timers |
| R0E | 44879-44902 | timers |
| H7X | 44903-44931 | units |
| R1E | 45004-45059 | units, timers |
| JNX | 45106-45180 | units, timers |
| R2E | 45221-45276 | units, timers |
| JHX | 45292-45365 | stock |
| R3E | 45395-45427 | units, timers |
| JQX | 45471-45528 | timers |
| F3E | 45572-45582 | timers |
| J0X | 45732-45745 | timers |
| R4E | 45839-45865 | timers |
| J9X | 45929-45956 | timers |
| KXX | 45960-45975 | units |
| R6E | 46013-46057 | units, timers |
| KCX | 46076-46136 | timers |
| KPX | 46165-46170 | timers |
| KUX | 46218-46224 | units |
| K0X | 46299-46309 | timers |
| K1X | 46310-46317 | units |
| K9X | 46423-46443 | timers |
| R8E | 46461-46473 | timers |
| LRX | 46503-46518 | timers |
| LNX | 46541-46553 | timers |
| IVE | 46634-46653 | timers |
| LLX | 46711-46725 | units |
| IXE | 46869-46907 | units, timers |
| L3X | 47006-47030 | timers |
| MEX | 47101-47142 | timers |
| MAX | 47186-47306 | units, timers |
| IOE | 47307-47316 | timers |
| MDX | 47317-47327 | units |
| MHX | 47380-47443 | units, timers |
| IRE | 47444-47473 | timers |
| MKX | 47477-47496 | units |
| IIE | 47513-47525 | timers |
| MPX | 47551-47562 | units |
| IAE | 47613-47634 | timers |
| FQE | 47652-47687 | units, timers |
| INE | 47706-47741 | units, timers |
| F4E | 47761-47770 | timers |
| MYX | 47782-47792 | timers |
| M2X | 47850-47887 | units |
| IBE | 47888-47905 | timers |
| M3X | 47906-47913 | units |
| M8X | 47975-48004 | units, timers |
| M9X | 48010-48039 | units, timers |
| ICE | 48134-48200 | units, timers |
| IFE | 48251-48265 | units |
| PNX | 48275-48278 | tech |
| PCX | 48305-48321 | timers |
| PGX | 48349-48366 | timers |
| PKX | 48382-48388 | units |
| PLX | 48396-48403 | units |
| IGE | 48404-48449 | units |
| IHE | 48480-48505 | units |
| IJE | 48586-48610 | timers |
| P4X | 48655-48675 | timers |
| P5X | 48676-48695 | timers |
| IKE | 48708-48714 | units |
| P9X | 48748-48772 | timers |
| QOX | 48790-48805 | units |
| ILE | 48811-48828 | units |
| IME | 48960-48996 | units, timers |
| IPE | 49024-49042 | units |
| QJX | 49046-49052 | units |
| QKX | 49053-49058 | units |
| QQX | 49095-49113 | timers |
| IQE | 49114-49142 | units |
| QTX | 49176-49196 | timers |
| QWX | 49208-49217 | timers |
| Q_X | 49238-49285 | units |
| ISE | 49286-49322 | units, timers |
| IUE | 49336-49376 | units |
| IWE | 49419-49456 | units |
| Q5X | 49457-49479 | units |
| FKE | 49531-49555 | timers |
| IYE | 49584-49608 | timers |
| SIX | 49662-49680 | timers |
| F7E | 49769-49777 | timers |
| IZE | 49859-49899 | units, timers |
| SMX | 49929-49975 | timers |
| STX | 50008-50015 | timers |
| SZX | 50126-50183 | timers |
| I_E | 50184-50200 | timers |
| I0E | 50217-50253 | units, timers |
| I1E | 50365-50414 | units, timers |
| TAX | 50426-50435 | timers |
| I3E | 50478-50491 | timers |
| I2E | 50530-50551 | units, timers |
| I4E | 50594-50644 | timers |
| TQX | 50735-50754 | timers |
| TYX | 50816-50831 | timers |
| T_X | 50837-50893 | units |
| T3X | 50894-50904 | timers |
| T4X | 50905-50947 | units |
| I5E | 50959-51012 | units, timers |
| I6E | 51026-51039 | timers |
| T8X | 51059-51067 | units |
| UVX | 51099-51122 | timers |
| I7E | 51123-51144 | units, timers |
| UXX | 51176-51216 | units, timers |
| UIX | 51277-51329 | timers |
| I8E | 51359-51368 | timers |
| UKX | 51392-51414 | timers |
| UPX | 51437-51448 | tech |
| UQX | 51455-51459 | time |
| USX | 51460-51484 | timers, time |
| I9E | 51547-51590 | units, timers |
| U5X | 51607-51639 | timers |
| AVE | 51666-51700 | timers |
| AEE | 51721-51754 | timers |
| WOX | 51758-51766 | units |
| AXE | 51838-51873 | timers |
| GWE | 51911-51928 | timers |
| WHX | 51940-51990 | units |
| AOE | 51991-52017 | units, timers |
| WMX | 52042-52073 | units, timers |
| AAE | 52074-52136 | units, timers, hero |
| ARE | 52211-52250 | units, timers |
| WZX | 52327-52354 | timers |
| W3X | 52422-52527 | timers |
| W7X | 52528-52542 | timers |
| YVX | 52585-52596 | units, timers |
| ABE | 52766-52785 | units, timers |
| YYX | 52842-52848 | units |
| Y6X | 52943-52978 | units |
| Y7X | 52979-53004 | timers |
| Y8X | 53005-53020 | units |
| Y9X | 53028-53068 | units |
| ZVX | 53069-53094 | timers |
| ZRX | 53135-53170 | units |
| ZCX | 53274-53290 | units |
| ZDX | 53291-53310 | units |
| ZLX | 53348-53363 | units, timers |
| ZTX | 53407-53412 | timers |
| ZZX | 53459-53464 | timers |
| Z_X | 53465-53486 | units |
| Z2X | 53513-53593 | timers |
| Z4X | 53594-53636 | timers |
| AHE | 53754-53809 | units, timers |
| VRO | 53829-53853 | units |
| VCO | 53867-53879 | units |
| VDO | 53880-53909 | timers |
| VHO | 53910-53933 | timers |
| VJO | 53965-53973 | units |
| AKE | 54064-54089 | timers |
| ALE | 54139-54181 | timers |
| FPE | 54240-54258 | timers |
| AME | 54289-54321 | units, timers |
| V1O | 54343-54372 | units, timers |
| V2O | 54373-54419 | timers |
| V4O | 54439-54461 | units, timers |
| EXO | 54548-54568 | timers |
| AQE | 54609-54631 | units, timers |
| ECO | 54651-54662 | units |
| EJO | 54744-54793 | units, timers |
| ASE | 54794-54810 | timers |
| BZV | 54878-54890 | timers |
| EMO | 54891-54903 | timers |
| ETO | 54925-54950 | timers |
| ATE | 55007-55040 | units, timers |
| E_O | 55067-55073 | timers |
| B_V | 55198-55210 | timers |
| E1O | 55211-55223 | timers |
| E3O | 55238-55245 | timers |
| E7O | 55293-55311 | units |
| XEO | 55398-55411 | timers |
| XOO | 55424-55450 | units |
| XNO | 55506-55531 | units, timers |
| XBO | 55535-55560 | units |
| XDO | 55597-55617 | units, timers |
| AZE | 55641-55674 | units |
| XKO | 55729-55741 | timers |
| XLO | 55742-55762 | units, timers |
| XMO | 55763-55775 | units, timers |
| XZO | 55834-55874 | units |
| X0O | 55884-55887 | tech |
| A_E | 55903-55948 | units |
| X5O | 55984-55990 | timers |
| X8O | 56016-56040 | units |
| OXO | 56090-56100 | timers |
| OAO | 56127-56193 | units, timers |
| OBO | 56222-56254 | units, timers |
| F9E | 56352-56382 | timers |
| OKO | 56408-56457 | units |
| OLO | 56458-56525 | timers |
| A1E | 56526-56548 | timers |
| OUO | 56566-56573 | timers |
| OZO | 56600-56611 | timers |
| O_O | 56612-56639 | units, timers |
| O8O | 56734-56866 | units |
| A2E | 56867-56919 | units, timers |
| A3E | 56956-56994 | units |
| RBO | 57032-57052 | timers |
| RCO | 57053-57062 | timers |
| RKO | 57155-57190 | units |
| R3O | 57401-57445 | units |
| R5O | 57446-57452 | timers |
| R6O | 57453-57496 | timers |
| R7O | 57497-57504 | timers |
| R8O | 57505-57553 | units, timers |
| IVO | 57554-57571 | timers |
| IOO | 57602-57638 | units |
| A4E | 57654-57685 | units |
| IBO | 57689-57712 | units |
| ITO | 57888-57934 | units, timers |
| G1E | 58010-58055 | units, timers, tech |
| I0O | 58059-58081 | units |
| I1O | 58082-58111 | timers |
| I4O | 58140-58172 | units |
| A7E | 58173-58194 | units, timers |
| A9E | 58237-58246 | timers |
| NEE | 58373-58398 | units, timers |
| NVE | 58526-58565 | units, timers |
| GEE | 58603-58619 | timers |
| AKO | 58635-58652 | timers |
| NXE | 58685-58710 | timers |
| NOE | 58719-58730 | timers |
| ATO | 58799-58833 | timers |
| AYO | 58847-58889 | units |
| NRE | 58890-58918 | units, timers |
| NIE | 58964-59010 | timers |
| A9O | 59072-59092 | units |
| NXO | 59138-59175 | timers |
| NBE | 59214-59260 | units |
| NAE | 59273-59276 | units |
| NFO | 59352-59368 | timers |
| G7E | 59384-59390 | tech |
| NLO | 59428-59440 | timers |
| NCE | 59473-59515 | timers |
| NQO | 59524-59535 | timers |
| NTO | 59561-59571 | timers |
| NDE | 59600-59618 | timers |
| NFE | 59639-59671 | timers |
| N1O | 59717-59793 | units |
| N8O | 59794-59814 | units, timers |
| BXO | 59895-59929 | units, timers |
| BAO | 59952-59975 | timers |
| BCO | 60067-60111 | timers |
| NGE | 60127-60163 | units |
| BLO | 60200-60252 | units |
| NJE | 60253-60288 | units, timers |
| BSO | 60316-60358 | units |
| BWO | 60393-60411 | units, timers |
| B0O | 60447-60469 | timers |
| NKE | 60536-60566 | timers |
| B6O | 60589-60597 | timers |
| B8O | 60641-60672 | timers |
| CVO | 60683-60688 | timers |
| CEO | 60689-60730 | timers |
| CRO | 60816-60838 | timers |
| GRE | 60941-60957 | timers |
| CKO | 61074-61080 | timers |
| NLE | 61089-61117 | timers |
| CQO | 61121-61151 | units |
| CWO | 61163-61170 | hero |
| CYO | 61171-61223 | units |
| C1O | 61224-61235 | timers |
| C3O | 61255-61333 | units |
| C4O | 61334-61364 | timers |
| C8O | 61394-61403 | timers |
| C9O | 61416-61427 | units |
| DVO | 61428-61480 | units |
| NME | 61481-61529 | units, timers |
| DXO | 61553-61600 | timers |
| DNO | 61679-61718 | timers |
| DFO | 61862-61873 | timers |
| DGO | 61874-61892 | timers |
| NQE | 61946-61970 | units, timers |
| DPO | 62085-62104 | timers |
| GIE | 62121-62130 | timers |
| DTO | 62142-62155 | timers |
| DYO | 62236-62293 | units, timers |
| D3O | 62378-62406 | timers |
| B2V | 62456-62467 | timers |
| D5O | 62468-62480 | timers |
| D6O | 62481-62517 | units |
| D8O | 62525-62535 | units |
| GAE | 62617-62663 | units, timers |
| FNO | 62714-62751 | timers |
| FFO | 62755-62786 | units |
| FGO | 62787-62809 | timers |
| FLO | 62853-62867 | units |
| FQO | 62868-62895 | units |
| FWO | 62919-62926 | timers |
| B0V | 63076-63091 | timers |
| F3O | 63115-63122 | timers |
| F9O | 63220-63227 | timers |
| GVO | 63247-63261 | units |
| GEO | 63267-63291 | units |
| RSE | 63331-63338 | timers |
| RPE | 63359-63379 | timers |
| ducc | 63398-63410 | timers |
| GCO | 63458-63529 | units, timers |
| GHO | 63555-63602 | units |
| GMO | 63621-63628 | timers |
| GQO | 63635-63648 | units |
| GSO | 63649-63656 | timers |
| NWE | 63684-63698 | timers |
| G3O | 63762-63790 | timers |
| R5E | 63806-63812 | timers |
| HIO | 63912-63926 | timers |
| HNO | 63927-63983 | units, timers |
| NYE | 63984-64003 | timers |
| HGO | 64022-64065 | units |
| N_E | 64156-64169 | timers |
| HTO | 64218-64242 | timers |
| N0E | 64243-64264 | units |
| N1E | 64428-64492 | units, timers |
| H8O | 64524-64558 | timers |
| N2E | 64559-64585 | units |
| JHO | 64749-64774 | units, timers |
| N3E | 64857-64939 | units, timers |
| J_O | 65020-65047 | timers |
| N5E | 65127-65169 | units, timers |
| N6E | 65286-65320 | units, timers |
| KOO | 65336-65343 | timers |
| KNO | 65375-65398 | units |
| N7E | 65399-65416 | units, timers |
| KBO | 65417-65437 | timers |
| KCO | 65438-65465 | units, timers |
| N8E | 65514-65543 | units, timers |
| N9E | 65560-65578 | timers |
| KPO | 65597-65616 | units, timers |
| KWO | 65642-65654 | units |
| K0O | 65690-65711 | timers |
| K4O | 65776-65804 | units, timers |
| K6O | 65869-65898 | units, timers |
| LXO | 65952-65964 | timers |
| LRO | 65969-66012 | units |
| BVE | 66013-66034 | units, timers |
| LAO | 66052-66078 | timers |
| BEE | 66150-66193 | units, tech |
| LMO | 66247-66263 | timers |
| BXE | 66305-66352 | units, timers |
| BOE | 66387-66409 | timers |
| LYO | 66410-66418 | units |
| BRE | 66442-66463 | timers |
| BAE | 66464-66544 | timers |
| L3O | 66559-66574 | timers |
| MQO | 66916-66923 | timers |
| M0O | 67086-67122 | timers |
| M2O | 67123-67152 | units, timers |
| M6O | 67195-67232 | units, timers |
| PVO | 67266-67382 | timers |
| PRO | 67453-67468 | timers |
| PJO | 67578-67595 | units |
| BBE | 67620-67638 | units, timers |
| PSO | 67710-67849 | hero |
| QFO | 67881-67913 | timers |
| QKO | 67956-68036 | units |
| QQO | 68037-68282 | units |
| SDO | 68320-68330 | hero |
| SGO | 68338-68359 | hero |
| SLO | 68406-68411 | timers |
| SMO | 68412-68440 | timers |
| SPO | 68441-68464 | units |
| BCE | 68560-68573 | units, timers |
| SUO | 68574-68588 | units |
| BDE | 68589-68603 | timers |
| BFE | 68670-68687 | timers |
| S1O | 68707-68794 | units |
| S3O | 68795-68829 | units, timers |
| S2O | 68830-68842 | timers |
| BHE | 68843-68852 | timers |
| S8O | 68884-68890 | timers |
| FFE | 68998-69016 | timers |
| TBO | 69080-69103 | timers |
| TCO | 69104-69119 | units, timers |
| RTE | 69138-69149 | units |
| G5E | 69189-69223 | units, timers |
| BKE | 69224-69262 | units |
| BLE | 69414-69457 | units, timers |
| BME | 69499-69529 | timers |
| TUO | 69530-69549 | timers |
| TWO | 69550-69559 | timers |
| T0O | 69616-69640 | timers |
| BPE | 69669-69686 | units |
| BSE | 69833-69869 | timers |
| UOO | 69927-69960 | units, timers |
| BTE | 70051-70064 | timers |
| UFO | 70157-70190 | units, timers |
| ULO | 70229-70236 | timers |
| FGE | 70306-70324 | timers |
| BUE | 70473-70544 | units, timers |
| BWE | 70619-70671 | units, timers |
| BYE | 70714-70731 | timers |
| WXO | 70741-70768 | timers |
| GBE | 70813-70819 | timers |
| WCO | 70844-70859 | timers |
| WJO | 70944-70982 | timers |
| B0E | 71115-71132 | timers |
| B1E | 71180-71195 | timers |
| WTO | 71232-71242 | timers |
| WYO | 71282-71293 | timers |
| B3E | 71334-71366 | units |
| W1O | 71388-71452 | units, timers |
| W5O | 71453-71462 | timers |
| W6O | 71463-71511 | units |
| B4E | 71560-71579 | units, timers |
| B5E | 71678-71701 | units |
| YBO | 71716-71730 | units |
| YLO | 71824-71832 | timers |
| YMO | 71833-71844 | units |
| YUO | 71901-71935 | units, timers |
| YWO | 71944-71986 | units |
| YYO | 71987-72025 | units |
| Y5O | 72091-72133 | timers |
| ZXO | 72230-72241 | timers |
| ZRO | 72257-72270 | units |
| B6E | 72303-72335 | timers |
| FVE | 72477-72518 | units, timers |
| B7E | 72556-72567 | units |
| ZKO | 72580-72609 | timers |
| GCE | 72624-72638 | timers |
| ZPO | 72672-72703 | timers |
| ZTO | 72720-72726 | timers |
| Z0O | 72783-72815 | units, timers |
| GDE | 72835-72844 | timers |
| Z2O | 72854-72863 | units |
| Z6O | 72911-73014 | units, timers |
| Z9O | 73015-73042 | timers |
| B8E | 73043-73058 | units, timers |
| VER | 73105-73126 | timers |
| VXR | 73127-73172 | units |
| VOR | 73173-73199 | units, timers |
| B9E | 73237-73268 | units |
| VNR | 73287-73307 | timers |
| VBR | 73308-73322 | units |
| VGR | 73391-73404 | timers |
| CVE | 73505-73525 | units, timers |
| VSR | 73526-73535 | units |
| V0R | 73661-73682 | timers |
| V2R | 73724-73737 | timers |
| V3R | 73738-73759 | units |
| CEE | 73980-74002 | units, timers |
| CXE | 74024-74079 | timers |
| EFR | 74147-74175 | units, timers |
| EKR | 74200-74205 | timers |
| EMR | 74249-74268 | timers |
| EQR | 74286-74300 | timers |
| COE | 74341-74384 | units, timers |
| CRE | 74391-74422 | units |
| CIE | 74442-74463 | timers |
| FIE | 74478-74497 | timers |
| E0R | 74560-74573 | timers |
| E3R | 74595-74630 | units |
| E4R | 74631-74655 | timers |
| E6R | 74659-74712 | units |
| E8R | 74713-74769 | units |
| CAE | 74842-74898 | units, timers |
| XAR | 74952-74977 | timers |
| XJR | 75088-75115 | timers |
| XKR | 75116-75127 | units |
| CNE | 75146-75161 | units |
| XYR | 75258-75288 | timers |
| CCE | 75289-75301 | timers |
| X0R | 75361-75399 | units, timers |
| X5R | 75414-75420 | timers |
| OXR | 75469-75596 | units |
| OIR | 75597-75653 | units, timers |
| OFR | 75764-75786 | timers |
| OJR | 75811-75827 | timers |
| OQR | 75874-75893 | timers |
| OUR | 75934-75965 | timers |
| CDE | 75969-76005 | units |
| O3R | 76109-76116 | timers |
| CFE | 76127-76141 | units, timers |
| FCE | 76204-76243 | units, timers |
| RVR | 76303-76310 | timers |
| CHE | 76367-76380 | units |
| ROR | 76392-76403 | timers |
| RAR | 76422-76440 | units |
| RDR | 76534-76540 | units |
| RGR | 76643-76674 | units, timers |
| CKE | 76772-76816 | units, timers |
| RZR | 76889-76899 | timers |
| CLE | 76941-76970 | timers |
| R1R | 76989-77011 | units, timers |
| R3R | 77020-77068 | units |
| CME | 77069-77097 | units, timers |
| CPE | 77134-77168 | units, timers |
| R7R | 77177-77187 | timers |
| IER | 77241-77292 | timers |
| IAR | 77315-77356 | timers |
| ICR | 77390-77420 | timers |
| IGR | 77459-77477 | timers |
| IMR | 77585-77607 | timers |
| CTE | 77661-77681 | units |
| IUR | 77738-77756 | timers |
| CSE | 77757-77789 | units |
| I8R | 78079-78097 | timers |
| GLE | 78125-78148 | timers |
| AVR | 78149-78171 | timers |
| ARR | 78198-78218 | units |
| CUE | 78254-78268 | units, timers |
| FEE | 78308-78346 | timers |
| CWE | 78385-78414 | timers |
| ASR | 78465-78473 | timers |
| AWR | 78507-78526 | units |
| AYR | 78527-78540 | timers |
| CYE | 78589-78600 | timers |
| A6R | 78643-78651 | timers |
| MUO | 78721-78728 | timers |
| NER | 78763-78784 | units, timers |
| C_E | 78850-78900 | units, timers |
| NCR | 78963-78987 | units, timers |
| NQR | 79077-79097 | timers |
| C0E | 79098-79108 | timers |
| NUR | 79147-79164 | timers |
| C2E | 79195-79205 | timers |
| NZR | 79206-79217 | timers |
| N3R | 79304-79331 | units |
| AQV | 79363-79385 | timers |
| N6R | 79415-79423 | timers |
| C3E | 79493-79512 | units, timers |
| BVR | 79519-79567 | units |
| BER | 79568-79589 | timers |
| BAR | 79680-79712 | units, timers |
| C4E | 79767-79791 | units, timers |
| BDR | 79820-79847 | timers |
| BFR | 79848-79866 | timers |
| BJR | 79895-79943 | units, timers |
| C6E | 79944-79955 | timers |
| GME | 80073-80087 | timers |
| BMR | 80127-80167 | timers |
| BQR | 80183-80193 | timers |
| C7E | 80226-80256 | units, timers |
| BWR | 80321-80344 | timers |
| C8E | 80411-80449 | timers |
| B3V | 80494-80505 | timers |
| B1R | 80506-80518 | timers |
| CER | 80628-80660 | units, timers |
| CRR | 80680-80710 | units |
| DXE | 80711-80748 | units, timers |
| DOE | 80752-80826 | units |
| CAR | 80848-80860 | timers |
| DRE | 80921-80951 | timers |
| CKR | 81039-81143 | units |
| DIE | 81144-81234 | units, timers |
| D4R | 81571-81644 | units, timers |
| D7R | 81680-81712 | timers |
| D9R | 81722-81727 | timers |
| FXR | 81790-81813 | timers |
| DAE | 81972-82024 | units, timers |
| FIR | 82098-82129 | timers |
| DNE | 82180-82198 | units, timers |
| FDR | 82221-82261 | units, timers |
| FFR | 82262-82294 | units, timers |
| FGR | 82295-82311 | timers |
| DBE | 82346-82381 | timers |
| FLR | 82405-82418 | timers |
| FSR | 82460-82489 | timers |
| FUR | 82496-82517 | units |
| B1V | 82568-82579 | timers |
| FYR | 82580-82592 | timers |
| DCE | 82614-82630 | units, timers |
| F3R | 82693-82738 | units, timers |
| DDE | 82818-82844 | timers |
| GVR | 82888-82902 | timers |
| GER | 82903-82943 | timers |
| GXR | 82944-82958 | timers |
| GNR | 83051-83088 | timers |
| DFE | 83089-83108 | timers |
| GCR | 83147-83169 | units, timers |
| A6V | 83202-83210 | timers |
| GJR | 83215-83265 | units |
| GKR | 83266-83290 | units, timers |
| DGE | 83353-83396 | units, timers |
| GTR | 83416-83432 | units |
| DHE | 83668-83738 | units, timers |
| GPE | 83830-83842 | timers |
| HVR | 83988-84041 | timers |
| HXR | 84066-84080 | units |
| HAR | 84188-84232 | timers |
| DJE | 84233-84259 | timers |
| HBR | 84293-84338 | units |
| DKE | 84386-84411 | units, timers |
| HMR | 84453-84478 | units, timers |
| HQR | 84521-84566 | timers |
| DLE | 84567-84579 | timers |
| HWR | 84623-84665 | timers |
| H7R | 84891-84924 | timers |
| JVR | 84987-85031 | units, timers |
| JIR | 85081-85090 | timers |
| JHR | 85198-85203 | timers |
| JJR | 85204-85270 | units, timers |
| JUR | 85314-85319 | timers |
| JZR | 85348-85363 | timers |
| J2R | 85437-85459 | timers |
| J5R | 85532-85567 | units, timers |
| J9R | 85610-85626 | units |
| KFR | 85804-85814 | timers |
| KKR | 85860-85929 | timers |
| GQE | 86459-86473 | timers |
| K6R | 86504-86555 | units, timers |
| DME | 86650-86670 | units, timers |
| LFR | 86728-86763 | timers |
| LJR | 86836-86892 | units, timers |
| DPE | 86893-86907 | timers |
| LMR | 86965-86983 | units, timers |
| LUR | 87104-87113 | timers |
| L3R | 87212-87249 | units, timers |
| DTE | 87391-87432 | timers |
| MOR | 87455-87467 | timers |
| MBR | 87517-87522 | timers |
| MCR | 87523-87553 | timers |
| MLR | 87697-87748 | timers |
| DUE | 87798-87817 | timers |
| MSR | 87844-87856 | timers |
| DYE | 87932-87950 | timers |
| M5R | 88034-88048 | units |
| M8R | 88100-88146 | timers |
| PRR | 88232-88284 | units, timers |
| D0E | 88285-88311 | timers |
| PCR | 88333-88348 | timers |
| PHR | 88366-88373 | units |
| PLR | 88389-88530 | timers |
| D1E | 88531-88578 | units, timers |
| P4R | 88674-88695 | timers |
| P6R | 88716-88733 | timers |
| QOR | 88839-88854 | units, timers |
| D2E | 88896-88919 | units, timers |
| QDR | 88995-89017 | units, timers |
| QJR | 89080-89107 | timers |
| QLR | 89129-89142 | timers |
| GSE | 89155-89171 | units, timers |
| QQR | 89183-89196 | timers |
| QWR | 89268-89290 | timers |
| QYR | 89291-89343 | units |
| Q1R | 89351-89370 | units, timers |
| Q5R | 89400-89426 | units, timers |
| SVR | 89502-89526 | timers |
| D3E | 89616-89640 | timers |
| SRR | 89674-89687 | timers |
| SAR | 89717-89734 | timers |
| D4E | 89735-89751 | units, timers |
| D5E | 89836-89853 | timers |
| FNE | 89928-89955 | units, timers |
| SHR | 89956-89981 | timers |
| SMR | 90020-90065 | units, timers |
| D6E | 90066-90160 | units, timers, hero |
| GTE | 90195-90204 | timers |
| SQR | 90205-90213 | timers |
| SZR | 90335-90348 | timers |
| S_R | 90361-90367 | units |
| S1R | 90398-90416 | timers |
| D7E | 90476-90499 | units, timers |
| S7R | 90511-90517 | units |
| D8E | 90658-90707 | timers |
| TTR | 90916-90945 | units, timers |
| TYR | 90993-91007 | timers |
| T_R | 91016-91030 | timers |
| T3R | 91082-91088 | timers |
| T4R | 91089-91123 | units |
| UER | 91253-91324 | timers |
| URR | 91353-91372 | timers |
| UFR | 91454-91470 | timers |
| ULR | 91550-91555 | timers |
| UMR | 91556-91587 | timers |
| USR | 91628-91638 | timers |
| UYR | 92213-92220 | timers |
| UZR | 92229-92251 | units |
| WXR | 92739-92796 | timers |
| RDE | 92797-92803 | units |
| RFE | 92804-92810 | units |
| RKE | 92811-92817 | units |
| FZE | 92818-92829 | units |
| RGE | 92830-92843 | units |
| RHE | 92844-92861 | units |
| RJE | 92862-92875 | units |
| FYE | 92876-92882 | units |
| FUE | 92883-92889 | units |
| FWE | 92890-92897 | units |
| F_E | 92898-92904 | units |
| F0E | 92905-92911 | units |
| RLE | 92912-92918 | units |
| RME | 92919-92925 | units |
| FSE | 92926-92940 | units |
| FTE | 92941-92964 | units |
| WBR | 93111-93119 | timers |
| WCR | 93120-93161 | units |
| WHR | 93240-93268 | units, timers |
| WKR | 93307-93335 | units, timers |
| WMR | 93347-93357 | timers |
| W4R | 93587-93596 | units |
| W8R | 93633-93677 | timers |
| YUR | 94316-94342 | timers |
| Y0R | 94401-94412 | timers |
| Y6R | 94655-94676 | timers |
| Y7R | 94677-94686 | timers |
| ZVR | 94698-94720 | units |
| ZER | 94721-94833 | units |
| ZGR | 98052-98073 | timers |
| ZWR | 98216-98431 | timers, tech |
| VRI | 98792-98805 | timers |
| NXJ | 99122-99129 | units |
| NNJ | 99147-99153 | tech |
| NHJ | 99176-99182 | units |
| BMJ | 99409-99707 | units, stock, time, hero |
| main | 99882-102203 | units, timers, time, tech |

## 2. Literal facts by class

### 2.1 Clock and day/night

- Start time of day: `6.` (30033 in `PRE`; 100916 in `main`). Dawn.
- Time-of-day scale: `.5` (100918 in `main`, before start); `.625` (30035 in `PRE`, at game start, overrides `.5`).
- Time of day set to `0` in `USX` (51480) when its flag is set. Not a start value.
- Suspend time of day `false` at start (100917, 30034). Map init is `main`.
- Clock length is not in the map. The script sets a rate, not a day or night duration. Our sim keeps 300 s day and 300 s night (`rust/crates/sim/src/lib.rs:70-71`); the map does not state those, so they stay as is.

### 2.2 Timers (literal intervals)

- Game clock (section 1.2): 30 s x3, 60 s, 180 s one-shot, 240 s, 450 s, 600 s, 849 s, 1759 s, 2669 s x2 (one-shot), plus 75 s one-shot `rR` (101706), 5 s (46119), 6 s (24615), 2.6 s (62800).
- Literal 0-interval one-shots and `.02`-`.5` sub-second timers are UI and effect ticks (121 of 141 timer registrations use a literal interval; the rest use variables).
- Waves: the 30 s handler is the only literal 30 s spawn path found. The 7-run gate (`wT`) gives a 210 s secondary cadence (26201, 26297).

### 2.3 Stock, levels and XP

- Starting gold and lumber: no literal. `SetPlayerState` gold and lumber calls use variables (JT, Wz, AQJ) or the save string (99427). Needs the interpreter.
- Literal stock value set to 0: 10302, 37912-37913, 37952-38684 (reset lines), 38757-38766 (all six players). These are resets, not starting values.
- Literal gold deduction: `-1` per player with no hero, every 30 s clock cycle (30101, 30105).
- Hero level: literal 10 (`0xA`) in `SVE` (31172, 31176). Start level: not literal (`NMJ` at 99643).
- Literal XP: 140 (61167), `0xF8` (42683, 248), `700+135*i` (25693, variable).
- Stock count `n137` x3 for two stocks in run 1 of `JAE` (26224-26225).

### 2.4 Units and items

- `CreateUnit` and `CreateUnitAtLoc`: 857 calls. 133 have literal coordinates. 224 distinct literal raw codes. Most used literal codes: `e00E` (229 calls, owner-relative coordinates from `GetUnitX`/`GetUnitY` in most), `n0DC` (22), `uzig` (22), `emow` (21), `n0EO` (12), `o02B`, `ncop`, `e01V`, `e00S`, `ncp2`.
- The wave units are not literal: lane units use `ji`/`id` variables, and the extra units are `umtw` and `u00R` (26300, 26302).
- `CreateItem` and `UnitAddItemById`: 157 calls. Literal item codes found in more than one call: `I02M` (7), `I00J` (4), `I0U0` (4), `I0RS`, `I007`, `I006`, `I00K`, `I008`, `I0QR` (2 each). Most item calls take a variable code.
- Literal unit placements (133 calls with literal coordinates) are start, camp or dummy placements. Terrain, pathing, doodad and start positions are excluded from this run (another lane owns them).
- Play area literal: `Rect(-7040,-7680,7040,7168)` at 100915 (in `main`). Bounds, excluded with terrain.

### 2.5 Spawn counts

- Lane wave: per run of `JAE` (30 s), the lane units spawn once per branch. The per-run count is not literal; it comes from the branch structure at 26227-26296.
- Extra spawn block every 7th run (`wT`, 26297 on). Count not literal.
- Siege: no literal 300 s or 4800-tick period in the script. Our `SIEGE_STARTS` and `SIEGE_PERIOD` (4800 ticks) are not stated by the map (see `docs/CONVERGENCE_688G.md` section 8).

## 3. Decisions for our data

| Literal (line) | Our data | Result |
| --- | --- | --- |
| Wave period 30 s (30050, 26201) | `scenarios/moba/waves.osd` line 11: `creep_wave_*` period 480 ticks (30 s at 16 ticks/s) | Matches. No change. |
| First wave at game start (30051 `TriggerExecute`) | `waves.osd` creep abilities with no initial cooldown | Matches. No change. |
| 210 s extra block (26201, 26297) | none | Units `umtw`/`u00R` not identified and not a stated siege period. Needs the interpreter or unit identity. Not set. |
| Siege 5 min (none in script) | `waves.osd` lines 12-14, 4800 ticks | Not stated by the script. Kept unchanged. |
| Start time of day 6.0 and scale .625 (30033, 30035) | no start clock in `scenarios/moba`; day and night lengths are rust constants | No data field. Not changed. The rust sim is not in this lane. |
| Creep camp filter on (100920) | no data field | Not changed. |
| Tech swaps every 450 s (26536-26545) | no data field for periodic tech | Not changed. Needs a data home. |
| Starting gold, lumber, level | not literal | Needs the interpreter (runtime and save string). |
| Literal item codes (section 2.4) | `scenarios/moba/items.osd` | Not changed: item names and effects are not stated in the script. |
| Literal unit placements (section 2.4) | start and camp placements (excluded lane) | Not changed in this run. |
| Play rect (100915) | terrain and bounds (excluded lane) | Not changed. |

Values set in data: none. Every literal that the script states directly either already matches our data, or has no data field. The one differing period (siege 5 minutes) is not stated by the script, so it stays.

## 4. Not verified

- The identity of `umtw`, `u00R` and `n137`.
- The effect of `KKE`, `G0V`, `G1V`, `PXE`, `JXE` (bodies not read).
- Whether the 30 s handler's lane branches match `waves.osd` lane splits (needs `JRE` read).
