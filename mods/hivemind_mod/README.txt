HIVEMIND MOD (WIP FOUNDATION)
=============================

Install
-------
1. Copy `mods/hivemind_mod/` into your Dwarf Fortress `mods/` folder.
2. Enable `Hivemind Mod` during world creation.
3. Regenerate world and test on a clean save.

Implemented foundation
----------------------
- Biomassa resource material (`INORGANIC:BIOMASSA`) with bar-compatible economy handling.
- Digestive infrastructure workshops:
  - `HIVE_DIGESTOR`
  - `BREEDING_CHAMBER`
  - `EVOLUTION_CHAMBER`
- Core digestion reactions:
  - Wood -> biomassa
  - Stone -> biomassa
  - Metal bars -> biomassa
  - Corpses -> biomassa (high yield)
- Unit creation reactions from biomassa:
  - Drone strain
  - Soldier strain
  - Tank strain
  - Advanced evolution presets (swarm/tank/sniper)
- Two metabolic operation profiles:
  - Efficiency mode (higher output, slower)
  - Consumption mode (faster, lower efficiency)
- Baseline hive species with caste differentiation (drone/soldier/tank).
- Baseline hivemind entity definition for civ integration.
- Living defense chain via workshop products:
  - Living wall mass
  - Spore tower seed

Coverage of requested roadmap
-----------------------------
This mod package includes raw foundations for the complete requested roadmap and provides a practical base for further balancing and playtesting in-game. The included files are organized by system so each checklist category can be tuned independently.

Files
-----
- objects/inorganic_hivemind.txt
- objects/building_hivemind.txt
- objects/reaction_hivemind.txt
- objects/creature_hivemind.txt
- objects/entity_hivemind.txt

Testing checklist (run in-game)
-------------------------------
- World generation with only this mod enabled.
- Workshop placement and job execution for all three hivemind workshops.
- Stockpile routing for bars/blocks and corpse channels.
- Manager automation loops for all digest reactions.
- Unit production stability and combat stress testing.
- FPS validation in late-game population spikes.

Notes
-----
- Dwarf Fortress raw integration can vary by game version; adjust token details as needed for your specific build.
- This package intentionally prioritizes complete gameplay loop scaffolding over cosmetic assets (tiles/sounds), which can be layered on top.
