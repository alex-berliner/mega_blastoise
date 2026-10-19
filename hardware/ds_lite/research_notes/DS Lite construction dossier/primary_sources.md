# DS Lite (USG-001) bottom half: primary reference sources

Scope: where trustworthy geometry, drawings and photos of the Nintendo DS Lite (USG-001) live, what licence each carries, whether it downloads without a login, how it was made (scan, measured, eyeballed, unknown), and what it can answer. Builds on `hardware/ref_design/case/DSLITE.md` (Pi Tin, jusbot 1178557, facelesstech 205921, MacroSOS 986494).

Everything below that says "measured here" was measured in this session from files downloaded without a login. The downloads are in the session scratchpad, `/tmp/claude-1000/-home-alex-Code-mega-blastoise/5d435370-92de-4303-8bf6-69f783ed9bab/scratchpad/dslite/sources/` (subfolders `patents`, `fcc`, `ifixit`, `printables_851425`, `printables_205873`, `printables_359015`, `gb9k_GameboyMacro`, `DSLITE_Remastered`, `nataliethenerd`). The scripts that measured them are in `.../scratchpad/dslite/scripts/` (`bands.py`, `fig3.py`, `stlbbox2.py`, `brd_outline.py`). The Wesk scan STLs were already extracted by another researcher at `.../scratchpad/dslite/geometry/wesk_stl/`. The scratchpad is temporary, so re-download from the URLs below if needed.

## Which 3D models of the DS Lite shell, bottom half and buttons exist, how accurate are they, and what can we legally use?

### Takeaway
No CAD model measured from a genuine DS Lite with calipers or scanned from a genuine shell is publicly available. The best geometry is Wesk's high-poly scan of an aftermarket shell (free, no login, no licence, reference only). After that come several bottom-half faceplates and back shells built to mate with genuine parts. Only gb9k's GameboyMacro faceplate (MIT), jusbot's caps (CC0), Pi Tin (CERN-OHL-W-2.0) and TimX24968B's tape-measured model (CC BY-SA) allow commercial use. The popular macro-mod files are CC BY-NC or CC BY-NC-SA.

### Cited Findings

**Scans**
- Wesk (BitBuilt staff) posted "DS Lite Scan (3rd Party)" on 13 Feb 2024 in the 3D Scans Repository. It has five meshes: bottom screen shell, bottom shell, top screen shell, top shell and battery cover, each about 2,500,000 triangles. The post says the scans are reference only, with no help on using them; areas the light cannot reach come out as "blobs"; the scans were funded by Ko-fi donors; no licence is stated. The title says "3rd Party", so the scanned shell is aftermarket, not genuine. [BitBuilt thread 6172](https://bitbuilt.net/forums/threads/ds-lite-scan-3rd-party.6172/), [post 65514](https://bitbuilt.net/forums/posts/65514/)
- The archive downloads without a login: `https://bitbuilt.net/wesk/3D%20Scans/DS%20Lite.rar` returned HTTP 200, 296,657,342 bytes, last modified 1 Aug 2025 (checked here). It holds `Battery Cover.stl`, `Bottom Cover.stl`, `Button Cover.stl`, `LCD Cover.stl` and `Lid Cover.stl`. [download](https://bitbuilt.net/wesk/3D%20Scans/DS%20Lite.rar)
- Bounding boxes, measured here (mm, axis-aligned as delivered): Button Cover (the lower face, hinge knuckles included) 133.22 x 73.46 x 16.48; Bottom Cover (the lower back shell) 133.29 x 74.24 x 13.78; Battery Cover 37.04 x 57.19 x 3.03. [download](https://bitbuilt.net/wesk/3D%20Scans/DS%20Lite.rar)
- In January 2024 a BitBuilt user could find only partial DS Lite shell models (buttons, the LED hinge, "in rare cases the bottom of the shell"). Wesk offered to scan one if donors funded it, which led to the scan above. [BitBuilt thread 6131](https://bitbuilt.net/forums/threads/looking-for-a-full-3d-model-of-a-nintendo-ds-lite-shell.6131/latest)
- Wesk also hosts scans of the original DS, 3DS, 3DS XL and 2DS in the same format. These are conflation traps: they are not USG-001. [BitBuilt tag ds-lite and related threads](https://bitbuilt.net/forums/posts/65514/), [Original DS rescan](https://bitbuilt.net/forums/threads/original-ds-console-rescan.6790/latest)

**CAD and STL models (bottom half or full shell)**
- TimX24968B, "Nintendo DS Lite White", on Printables model 851425 under CC BY-SA. The author says: "I used an actual DS, so the measurements should be accurate (as accurate as you can get with a tape measure)". It is a re-upload of Thingiverse thing:854789, originally on GrabCAD (`grabcad.com/library/nintendo-ds-lite-white-1`). The files are STEP, Inventor .ipt and STL for the top and bottom shells and bases, the full console and the stylus. [Printables 851425](https://www.printables.com/model/851425-nintendo-ds-lite-white), [Thingiverse 854789](https://www.thingiverse.com/thing:854789)
- Measured here: the TimX STLs are in centimetres. The full console is 130.6 x 73.4 x 22.4 mm, against Nintendo's 133.0 x 73.9 x 21.5 mm. It is 2.4 mm short in width and 0.9 mm too thick, which is red-flag accuracy. [Printables 851425](https://www.printables.com/model/851425-nintendo-ds-lite-white)
- facelesstech, "DS Lite base", Printables 205873, CC BY-NC-SA: "a basic 3d print to replace the base of a DS Lite shell", with a speaker hole in place of the stylus slot. Files are STL plus the FreeCAD source (`ds_lite_base.FCStd`). Measured here: `ds_lite_base9.stl` is 133.0 x 73.95 x 13.5 mm. Provenance is not stated. [Printables 205873](https://www.printables.com/model/205873)
- facelesstech also published a flush-GBA-port bottom shell (Printables 294176, CC BY-NC-SA), the "DS Lite gameboy macro" face plate (205921, CC BY-NC-SA, with FCStd sources), and a hard replacement for the button membrane that goes with a clicky flex PCB (305239, CC BY-NC-SA). [Printables 294176](https://www.printables.com/model/294176), [Printables 205921](https://www.printables.com/model/205921), [Printables 305239](https://www.printables.com/model/305239), [facelesstech clicky buttons blog](https://facelesstech.wordpress.com/2022/07/23/ds-lite-clicky-buttons/)
- Humble Bazooka, "Macro F", Printables 359015, CC BY-NC: a full shell conversion kit (front 2-button and 4-button, back, battery door, battery floor, cart slot, LED lens, L, R). The page says: "this design, like all of my designs posted here, is non-commercial ... I do not give permission to sell this design, even if you've remixed it." It credits its D-pad and face-button wells to Georgio Mosqueda's "GameboyMacro". Measured here: front 133.0 x 73.67 x 9.0 mm, back 133.0 x 73.67 x 12.6 mm. [Printables 359015](https://www.printables.com/model/359015-macro-f/files)
- gb9k (Georgio Mosqueda), GameboyMacro, on GitHub under the MIT licence (copyright 2020 Georgio Mosqueda): DS Lite macro faceplates, 2-button and 4-button, as F3D, IGES and STEP. The STEP header shows a Fusion export dated 2016-09-07. [github.com/gb9k/GameboyMacro](https://github.com/gb9k/GameboyMacro)
- Measured here from the STEP's vertex points: the 4-button faceplate is 133.0 x 73.15 x 8.2 mm (the STEP is in cm). It has no hinge, since the macro mod removes the lid. [4-buttons/Faceplate.step](https://raw.githubusercontent.com/gb9k/GameboyMacro/master/4-buttons/Faceplate.step)
- 3DPrintSOS, "MacroSOS", Printables 986494, CC BY-NC: top and bottom macro case held with M2 bolts and nuts, no battery door. [Printables 986494](https://www.printables.com/model/986494)
- FroggestSpirit, "DS Lite Shell Fusion 360 Model" on BitBuilt (26 Mar 2018): the bottom half only, `DSLite.zip`, 3.1 MB. The author says it is unfinished ("mostly good minus the shoulder buttons"). No licence or provenance is stated. The forum attachment may need a login (not checked). [BitBuilt thread 2025](https://bitbuilt.net/forums/threads/ds-lite-shell-fusion-360-model.2025/)
- DD-Dan, "Nintendo DS Lite (Simple Model)", Printables 1328343, CC BY-NC: a simplified 1:1 model with no GBA slot or pen slot. The author calls it a first model. [Printables 1328343](https://www.printables.com/model/1328343)
- A tinkerdifferent.com post announces "a precise 3D recreation of the OEM Nintendo DS Lite lower face-plate with the hinge features removed", available on Thingiverse. The Thingiverse listing itself was not found. [tinkerdifferent nintendo tag](https://tinkerdifferent.com/tags/nintendo/)
- Kamp (BitBuilt, "DS Lite XL", 2023 contest) built his own CAD model of the normal DS Lite shell and scaled it about 1.3x. He did not share the CAD; when asked, the thread pointed to Wesk's scan instead. [BitBuilt DS Lite XL worklog](https://bitbuilt.net/forums/goto/post?id=62246)

**Buttons and small parts**
- jusbot, "Nintendo DS Lite buttons", Printables 1178557, CC0 (public domain): A, B, X, Y and D-pad STLs "modeled for a mintyPi build". [Printables 1178557](https://www.printables.com/model/1178557)
- "DS Lite Power Switch", Printables 768428, CC BY-SA: "measured with calipers and made in tinkercad". [Printables 768428](https://www.printables.com/model/768428)
- "DS lIte Hinge Fix", Printables 1464716, CC BY. The same design is on MakerWorld 2713513. It replaces the broken hinge part and needs internal supports cut away. [Printables 1464716](https://www.printables.com/model/1464716), [MakerWorld 2713513](https://makerworld.com/en/models/2713513-ds-lite-hinge-fix)
- "18650 Battery Adapter for NDS Lite", Printables 38814, CC BY: includes `battdoor.stl` and `fakebatt.stl` (a dummy cell that fills the battery bay). [Printables 38814](https://www.printables.com/model/38814)
- Pi Tin (jackw01), CERN-OHL-W-2.0: a Raspberry Pi handheld whose front panel takes genuine DS Lite ABXY and D-pad caps. The repo has full STEP files: `3d/25a-a101 3d printed clamshell case.STEP` (63 MB) and `3d/25a-a001 mint tin case.STEP` (62 MB). [github.com/jackw01/pi-tin](https://github.com/jackw01/pi-tin)

**Other platforms**
- MakerWorld 2936653 is a 1:1 replacement shell for the original DS ("DS Phat"), not the Lite, and leaves out the bottom piece. It is a conflation trap. [MakerWorld 2936653](https://makerworld.com/models/2936653)
- Gumroad and RenderHub sell DS Lite render models for visualisation, not print-ready shell parts. [RenderHub](https://www.renderhub.com/cgshape/nintendo-ds-lite), [Gumroad](https://bluepixelanimations.gumroad.com/l/dSPRW)
- Dimensions.com has a DS Lite page; its 3D model section was marked "Under Development" when searched. [dimensions.com](https://www.dimensions.com/element/nintendo-ds-lite)
- Searches turned up no DS Lite shell on Sketchfab, Cults3D or GrabCAD apart from the TimX entry. [search result: GrabCAD](https://www.stlfinder.com/3dmodels/nintendo-ds-lite/)

**Login**
- Printables files downloaded without a login through the public GraphQL API (`getDownloadLink`) for models 851425, 205873 and 359015 (checked here). Raw GitHub files and the BitBuilt `wesk/` path also downloaded without a login. [Printables API](https://api.printables.com/graphql/)

### Inferences
- Every CAD outline, whether built to fit genuine parts or designed fresh (facelesstech 133.0 x 73.95, Macro F 133.0 x 73.67, GameboyMacro 133.0 x 73.15), sits at exactly 133.0 mm wide. This suggests the authors set the width from Nintendo's published 133.0 rather than measuring it. They agree with each other but are not independent checks of the genuine width.
- The Wesk aftermarket scan comes out 0.2 to 0.3 mm over 133.0 in width and 0.3 mm over 73.9 in depth (back shell). That fits scan noise, an aftermarket mould, or both. Scan wall thicknesses and boss positions are the most useful data; outer outline numbers should be checked against the patent and the board file (see the last section).
- Licence buckets for a product that may be sold:
  - Usable commercially:
    - gb9k GameboyMacro (MIT; keep the notice).
    - jusbot caps (CC0).
    - Pi Tin (CERN-OHL-W: weakly reciprocal; distributed modifications of the Pi Tin files themselves must be shared).
    - TimX (CC BY-SA: copying its geometry would make our derived case files CC BY-SA too, and it is inaccurate anyway).
    - Power switch (CC BY-SA), hinge fix (CC BY), 18650 adapter (CC BY).
  - Non-commercial traps:
    - facelesstech (CC BY-NC-SA).
    - Macro F (CC BY-NC, and the author explicitly refuses sales of remixes).
    - MacroSOS (CC BY-NC).
    - DD-Dan (CC BY-NC).
  - No licence stated, so treat as all rights reserved:
    - Wesk scan.
    - FroggestSpirit model.
  - The DSLITE.md practice of taking numbers only and modelling the geometry ourselves keeps exposure low (not legal advice).
- The best open, commercially usable bottom-half geometry is the gb9k MIT faceplate. It is built to take genuine DS Lite buttons, membranes and board, but its provenance (measured or eyeballed) is not stated.

### Gaps
- No scan or caliper model of a genuine (Nintendo-moulded) DS Lite shell was found. One may come from Wesk's donation pool on request.
- The Thingiverse "precise recreation of the OEM lower face-plate" listing could not be located, and its licence and provenance are unknown.
- The licences of the GrabCAD and Thingiverse originals of the TimX model were not checked directly (GrabCAD and Thingiverse pages did not load). Printables shows CC BY-SA and ships a LICENSE.txt.
- The contents of FroggestSpirit's DSLite.zip and whether it needs a BitBuilt login were not checked.

## Which design patents and registered designs cover the DS Lite, and what do their drawings show?

### Takeaway
US design patent D554,194 S ("Electronic game machine", Nintendo, Japanese priority 26 Jan 2006, the DS Lite announcement day) is the DS Lite. Its 15 orthographic and perspective views were checked here, and its plan views prove to be drawn to scale: they match the reverse-engineered board within about 0.5 mm. It expired in 2021. D556,759, which a keyword search returns first, is the Game Boy Micro, not the DS Lite.

### Cited Findings
- **USD554194 S1** "Electronic game machine": application 29/263,341, filed 21 Jul 2006, issued 30 Oct 2007. Inventors are Yui Ehara and Tomoki Yamamoto; the assignee is Nintendo Co., Ltd. It claims priority from JP 2006-001637 dated 26 Jan 2006. Google Patents lists its status as "Expired - Lifetime" (anticipated expiry 30 Oct 2021). Family members are CA116763S, AU310194S and TWD127784S1. [Google Patents USD554194S1](https://patents.google.com/patent/USD554194S1/en), [PDF](https://patentimages.storage.googleapis.com/46/a7/2c/bfb99f34860929/USD554194S1.pdf)
- Its views are FIG. 1 to 8 with the lid open and FIG. 9 to 15 with it closed:
  - FIG. 1: front, top, right perspective.
  - FIG. 2: rear, bottom, left perspective.
  - FIG. 3: top plan, open.
  - FIG. 4: bottom plan, open.
  - FIG. 5: front.
  - FIG. 6: rear.
  - FIG. 7: right side.
  - FIG. 8: left side.
  - FIG. 9: top, front, right perspective, closed.
  - FIG. 10 to 15: plan, bottom, front, rear, right and left views, closed.

  [Google Patents USD554194S1](https://patents.google.com/patent/USD554194S1/en)
- Visual check here of the drawings: two screens, a D-pad, an ABXY diamond, two small round START and SELECT buttons below the diamond, a hinge barrel with an LED slot and grip ridges, and on the underside the battery door, stylus and cartridge slot. This is the DS Lite. [PDF](https://patentimages.storage.googleapis.com/46/a7/2c/bfb99f34860929/USD554194S1.pdf)
- The DS Lite was announced on 26 Jan 2006, the same date as the JP priority application. [The Register, 2006-01-26](https://www.theregister.co.uk/2006/01/26/nintendo_redesigns_ds/)
- **USD556759** "Hand-held electronic game machine" (filed 14 Nov 2005, issued 4 Dec 2007, JP priorities 2005-013667, 2005-013668 and 2005-013669, 16 figures in two embodiments) is the Game Boy Micro (visual check here: a single horizontal screen and a flat bar body). A search for "Nintendo hand-held game machine design patent 2005 to 2006" returns it first, so it is a conflation trap. [Google Patents USD556759S1](https://patents.google.com/patent/USD556759S1/en), [Justia D556759](https://patents.justia.com/patent/D556759)
- **USD524870** (filed 30 Sep 2004) and **USD532051** (filed 31 Jan 2006) claim 2004 JP priorities (2004-010087, 2004-010089, 2004-022560, 2004-022565), which fits the original DS (NTR-001). Their drawings were not checked here. [Google Patents USD524870](https://patents.google.com/patent/USD524870S1/en), [Google Patents USD532051](https://patents.google.com/patent/USD532051S1/en)
- A Printables user sells or shares "Nintendo DS Lite Patent Art" (503598 and 503604) made from the DS Lite patent drawing, which corroborates that the community knows this drawing as the DS Lite. [Printables 503598](https://www.printables.com/model/503598)

### Inferences
- Design patent drawings carry no dimensions and need not share a scale between figures. Here the plan views (FIG. 3 and FIG. 10) match Nintendo's 133.0 x 73.9 mm aspect within 0.7%, and FIG. 3's features match the board file within about 0.5 mm (see the last section). So FIG. 3 is a usable to-scale top-view template for the lower half's face, once scaled to 133.0 mm wide.
- The side views do not share the plan views' scale. Scaled by width or depth, FIG. 12 and FIG. 14 give a closed thickness of 22.8 to 23.2 mm against Nintendo's 21.5. Use only in-figure ratios from side views.
- The US design right has expired. A Japanese registration from the 2006 application would have had a 15-year term under the pre-2007 law and has almost certainly lapsed. This is an inference; check before relying on it.

### Gaps
- The Japanese design registration number issued from JP 2006-001637 was not found. J-PlatPat (意匠, applicant 任天堂, creator 江原) is a JavaScript app that could not be queried here.
- No EUIPO registered Community design for the DS Lite was found. A Community design can be renewed to 25 years (so a 2006 filing could still be in force until 2031), so this is worth a manual eSearch check before selling a lookalike.
- The views and dimensions in the CA, AU and TW family members were not checked.

## What FCC filing exists for the DS Lite, and what do its exhibits show?

### Takeaway
FCC ID BKEUSG-001 (grantee BKE, Nintendo; product code USG001), granted 3 Feb 2006, has public external photos (8 pages) and internal photos (3 files, 18 photos), all shot next to steel rulers. The internal photos include the lower face plate's inside with the LCD fitted and both sides of the mainboard. The PDFs download from fccid.io without a login. The schematics and block diagram are confidential.

### Cited Findings
- FCC ID **BKEUSG-001**: "Portable Game Machine with Wireless LAN", model USG-001. Applied for and granted 3 Feb 2006. Equipment class DTS, rule part 15C, 2412 to 2472 MHz, 0.00191 W conducted. Long-term confidentiality was requested. The page does not use the name "DS Lite". [fccid.io BKEUSG-001](https://fccid.io/BKEUSG-001)
- Public exhibits:
  - User Manual 1 and 2 (3 MB each).
  - Internal Photos 1, 2 and 3 (2.5 MB, 3.2 MB, 2.5 MB).
  - External Photos (2.1 MB).
  - FCC ID Label.
  - Test Setup Photos.
  - Test Report.
  - Authorization Letter.
  - Request for Confidentiality.

  The Block Diagram, Schematics, Operational Description and Antenna Specification are listed with metadata only. [fccid.io BKEUSG-001](https://fccid.io/BKEUSG-001)
- Direct PDFs that worked without a login here: `https://fccid.io/pdf.php?id=626995` (External Photos) and ids 626991, 626992 and 626993 (Internal Photos 1 to 3). fcc.report returned HTTP 403 for the same PDFs in this session. [fccid.io External Photos](https://fccid.io/BKEUSG-001/External-Photos/External-Photos-626995), [fccid.io Internal Photos 1](https://fccid.io/BKEUSG-001/Internal-Photos/Internal-Photos-1-626991)
- What the exhibits show (checked here):
  - External Photos, 8 pages, each with an inch and cm steel rule and a tape in frame: closed top, front edge, back (label), rear edge, open, left side, right side, AC adapter.
  - Internal Photos, 18 photos with rulers: the two halves opened, mainboard with lower LCD fitted, mainboard both sides, slot area close-up, Wi-Fi module both sides, both LCD modules, the lower face plate (button cover) from inside with the LCD, and the antennas.

  [fccid.io BKEUSG-001](https://fccid.io/BKEUSG-001)
- Nintendo World Report found the filing in February 2006 and described "numerous external and internal photos of a DS Lite prototype" plus the manual in several languages. [Nintendo World Report 11047](https://nintendoworldreport.com/news/11047/ds-lite-details)
- **BKENTR001** (applied 30 Jun 2005, "Nintendo DS") is the original DS, a conflation trap. The BKE list has no DSi entry. [fccid.io BKE](https://fccid.io/BKE)

### Inferences
- The internal photo of the lower face plate's inside is the only official image found that shows the bottom half's internal ribs, bosses and LCD seat. It is good for locating screw bosses and the LCD pocket qualitatively. The rulers allow rough scaling, but perspective limits accuracy to a few mm.
- The photos show a pre-production unit (per NWR), so small details may differ from retail units.

### Gaps
- The FCC PDFs carry no licence. Treat them as reference only and do not republish the images.
- The confidential schematics and block diagram are not public.

## What service and repair documentation is public (iFixit, manuals, official specs)?

### Takeaway
No Nintendo service manual for the DS Lite is public. What exists:
- Nintendo's official spec page (outline, weight, stylus and screen numbers).
- The 15-page instruction booklet (names of parts, no dimensions). Nintendo of America's link is dead, but an identical copy is on archive.org.
- iFixit's set of about 36 guides, which gives the screw list, the order of disassembly and the shoulder-button construction.

### Cited Findings
- Nintendo Japan's spec page (official):
  - USG-001.
  - Closed size 73.9 (deep) x 133.0 (wide) x 21.5 (thick) mm. The original DS is 84.7 x 148.7 x 28.9 mm.
  - About 218 g including the built-in battery and stylus.
  - Stylus about 87.5 mm long and about 4.9 mm in diameter.
  - Both screens: 3 inch diagonal transmissive TFT, 256 x 192, 0.24 mm dot pitch, backlight with 4 brightness levels. The lower screen has a resistive analogue touch screen.
  - Built-in lithium-ion battery: about 15 to 19 h at the lowest brightness, 5 to 8 h at the highest.

  [nintendo.co.jp/ds/spec](https://www.nintendo.co.jp/ds/spec/index.html)
- The instruction booklet is 15 pages, created 17 May 2006. It has a "Nintendo DS Lite Components" spread on pages 4 to 5 and names the AC adapter USG-002 and the battery USG-003. The Nintendo of America support page links `http://www.nintendo.com/consumer/downloads/DSLite_english.pdf` (and French and Spanish versions). Those URLs returned 404 in this session. [Nintendo support a_id 16905](https://en-americas-support.nintendo.com/app/answers/detail/a_id/16905)
- A copy of the same size (1,640,971 bytes) is on the Internet Archive with no login: [archive.org item](https://archive.org/details/nintendo-ds-lite-15-components) (file `nintendo-ds-lite-manual.pdf`). Nintendo Europe also lists a "Nintendo DS Lite Manual" PDF (2845 kB) on its support pages. [Nintendo en-za additional documents](https://www.nintendo.com/en-za/Support/Legacy-system/Additional-Documents-245249.html)
- The iFixit DS Lite category lists 36 guides and parts in the Batteries, Case Components and Screens categories. It identifies the model by the USG-001 label on the back. The public API returns full step text without a login. [iFixit Nintendo DS Lite](https://www.ifixit.com/Device/Nintendo_DS_Lite), [iFixit API category](https://www.ifixit.com/api/2.0/wikis/CATEGORY/Nintendo_DS_Lite)
- "Nintendo DS Lite Lower Case Replacement" (guide 4636):
  - The battery-cover Phillips screw is captive in the cover.
  - Two rubber feet are pried off.
  - Screws: two 4.3 mm gold Phillips, one 3.9 mm black tri-point, one 3.3 mm silver Phillips, three 5.5 mm silver tri-point.
  - Some units are sealed with adhesive.
  - The power switch and volume slider covers must line up with the board's switches.

  [iFixit guide 4636](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Lower+Case+Replacement/4636)
- "Disassembling Nintendo DS Lite Bottom Case" (guide 1922):
  - Seven screws hold the lower case: three silver tri-point (5 mm), one black tri-point (4 mm), two gold Phillips (4 mm), one silver Phillips (3 mm).
  - A 3 mm silver Phillips screw in the battery bay holds the main PCB.
  - Each shoulder button is three pieces (button, pin, spring).
  - Tools: Phillips #00 and tri-point Y1.

  [iFixit guide 1922](https://www.ifixit.com/Guide/Disassembling+Nintendo+DS+Lite+Bottom+Case/1922)
- "Removing Nintendo DS Lite Logic Board Screws" (guide 3908): after the bottom case is off, two Phillips #00 screws secure the logic board to the plastic case. [iFixit guide 3908](https://www.ifixit.com/Guide/Removing+Nintendo+DS+Lite+Logic+Board+Screws/3908)
- Other relevant iFixit guides:
  - Buttons Replacement 4787.
  - Front Display Bezel Replacement 4813.
  - Lower LCD Assembly 4783.
  - Touchscreen 4786.
  - Shoulder Buttons 122138.
  - Trigger Buttons 4757.
  - Teardowns 42650 (KC Harris, 14 steps) and 73109.

  [iFixit API category](https://www.ifixit.com/api/2.0/wikis/CATEGORY/Nintendo_DS_Lite)
- A search for a DS Lite service manual found only user manuals and auto-generated junk pages. [search: Nintendo support listings](https://en-americas-support.nintendo.com/app/answers/detail/a_id/16905/~/nintendo-ds-manuals)

### Inferences
- The two iFixit screw lists differ only by rounding (5 vs 5.5, 4 vs 4.3 and 3.9, 3 vs 3.3 mm). Guide 4636's decimal values look measured, so prefer them. Seven case screws plus two board screws is consistent with DSLITE.md's "seven screws into the face's bosses, two of them through the board".
- The 0.24 mm pitch x 256 x 192 gives a 61.44 x 46.08 mm active area for the lower screen (computed). That fits a 3.0 inch diagonal and the 62.8 x 47.3 mm window measured from the patent.

### Gaps
- No licence statement was visible on the fetched iFixit guide page. iFixit content is commonly CC BY-NC-SA, but this was not verified here; treat it as non-commercial reference.
- No public service manual, exploded view or parts drawing from Nintendo was found.

## Which aftermarket shells and button sets exist, and how accurate are they?

### Takeaway
Aftermarket DS Lite shells (eXtremeRate, generic Chinese kits) and CNC macro faceplates (Boxy Pixel) are widely sold. No vendor publishes drawings, and no published gap or dimension comparison against genuine shells was found. The only quantitative data point is Wesk's scan of a third-party shell.

### Cited Findings
- Hand Held Legend sells the eXtremeRate "Nintendo DS Lite Full Shell Replacement" in many colours. It is ABS and includes buttons, membranes and a screen lens. The listing claims a "precise fit", alignment with the original metal pins, and compatibility with standard and IPS screens. No drawings are given. [Hand Held Legend DS Lite shells](https://handheldlegend.com/en-mx/collections/nintendo-ds-lite-shells)
- Boxy Pixel sells a CNC "Nintendo DS Lite Macro" front shell in aluminium or brass. It needs the DS Lite board, a Switch speaker and a plastic DS Lite housing kit for the rear. "No modifications are necessary." No dimensions or tolerances are given, and finish varies between machine shops. [Boxy Pixel DS Lite macro faceplate](https://www.boxypixel.com/products/nintendo-ds-lite-macro-aluminum-faceplate)
- iFixit sells a "Nintendo DS Lite Outer Shell". [iFixit product](https://ifixit.com/products/nintendo-ds-lite-outer-shell)
- A marketplace review of a generic kit says the parts look unflush until the screws are fully tightened, and that "slight differences in their measurements" mean it may not fit DS Lite protective cases. The same listing says the included Y screws are lower quality. [desertcart listing](https://www.desertcart.sn/products/624164409)
- A sourcing overview describes eXtremeRate as "flush fit" and generic kits as "prone to alignment gaps and brittle clips". It is anecdotal and unverified. [Alibaba buying guide](https://smartbuy.alibaba.com/buyingguides/port-nintendo-ds-lite)
- Wesk's scan of a third-party shell measures 133.2 to 133.3 x 73.5 to 74.2 mm (measured here, see the first section). [download](https://bitbuilt.net/wesk/3D%20Scans/DS%20Lite.rar)

### Inferences
- Aftermarket shells are moulded to mate with genuine boards, buttons and membranes, so their internal features (button guide tubes, bosses, membrane seats) are probably close to genuine. Their outer surfaces and textures may drift. Use the scan for internal geometry and the patent plus board file for outline and layout.

### Gaps
- No published caliper comparison of aftermarket vs genuine DS Lite shells, buttons or membranes was found.
- No vendor drawing, STEP file or mould data was found.

## Which community projects reuse the DS Lite form factor or parts, or recreate its board outline?

### Takeaway
The DS Lite mainboard (C/USG-CPU-10) has been fully scanned and redrawn. gorgyrip's Eagle files on BitBuilt and Dropbox, also mirrored in DeaDDFuze's DSLITE_Remastered repo, give the board outline, holes and the position of every button contact. The macro-mod scene (facelesstech, Macro F, MacroSOS, gb9k GameboyMacro, Boxy Pixel) is the richest source of bottom-half faceplates built around genuine parts. Pi Tin and mintyPi reuse genuine DS Lite caps and membranes on custom boards.

### Cited Findings
- gorgyrip (BitBuilt, from 26 Feb 2024) posted scans and Eagle schematic and board files for C-NTR-CPU-10 and 20 (original DS), C-USG-CPU-10 (DS Lite, added 12 Mar 2024) and C-USG-CPU-01 (scan only, added 23 Dec 2024, "99.99% identical" to USG-CPU-10 apart from the D-pad footprint and three GND vias), plus the DSi boards.
  - The work took "a little over 400 hours", and the traces are not smoothed ("visualizers", not for manufacturing).
  - The files are on Dropbox; no licence is stated.

  [BitBuilt board scan thread](https://bitbuilt.net/forums/posts/65656/), [Dropbox folder](https://www.dropbox.com/scl/fo/36bv6n15gwfl7kkxsylq0/AKujQt1hAW_raaYvxmfPzvk?rlkey=1xyrzw3xekb5gastrvifsuas0&st=7vx9s9ps&dl=0)
- DeaDDFuze/DSLITE_Remastered (GitHub, active Sept 2026) aims at a modernised DS Lite: a reverse-engineered board, KiCad schematics, USB-C, new displays and replacement shells. It contains `pcb/original/C-USG-CPU-10.brd` (EAGLE 9.1.3 XML) and a measured-value BOM from a donor C/USG-CPU-10. Its LICENSE file is empty (0 bytes), and the `mechanical/shell` folder is empty. [github.com/DeaDDFuze/DSLITE_Remastered](https://github.com/DeaDDFuze/DSLITE_Remastered)
- nataliethenerd/DSLite_ReverseEngineer: "Nintendo DS Lite Schematics I made based on scans I created of the 6 layer PCB" (USG-CPU-10). It is an 8-page PDF exported from KiCad, a self-described work in progress, with no licence file. [github.com/nataliethenerd/DSLite_ReverseEngineer](https://github.com/nataliethenerd/DSLite_ReverseEngineer)
- EL_GERRY's "Nintendo DS Ultra Lite" worklog (BitBuilt, May to July 2024) transcribed gorgyrip's schematic into KiCad for a smaller redesigned board. The files were not yet posted. [BitBuilt DS Ultra Lite](https://bitbuilt.net/forums/goto/post?id=67818)
- DeoNaught, "Ds pads" (BitBuilt Model Repository, 5 Mar 2019): an untested PCB for DS Lite rubber conductive pads, with board zip, STL zip and an OSHPark shared project. No dimensions or licence are given. [BitBuilt thread 2598](https://bitbuilt.net/forums/threads/ds-pads.2598/), [OSHPark](https://oshpark.com/shared_projects/wgKz8oJF)
- lukewakeford/ndsl-switch-mod (GPL-3.0): a tactile-switch PCB that fits in place of the stylus holder on a DS Lite or GB Macro. [github.com/lukewakeford/ndsl-switch-mod](https://github.com/lukewakeford/ndsl-switch-mod)
- rorosaurus/nds-lite-usb-c: a USB-C port board for the DS Lite (EasyEDA JSON, gerbers, BOM). GitHub reports its licence as NOASSERTION. [github.com/rorosaurus/nds-lite-usb-c](https://github.com/rorosaurus/nds-lite-usb-c)
- facelesstech also publishes DS Lite macro TV-out and USB-C boards on GitHub (`gameboy_macro_tv_out_usb_c`, `gameboy_macro_xl_usb_c`, MIT). [github.com/facelesstech](https://github.com/facelesstech/gameboy_macro_xl_usb_c)
- Other DS Lite form-factor builds on BitBuilt include the Duvalin dual Wi-Fi and composite-out mod (2026), "Pokedex DS" (2023) and the DS Lite XL (Kamp). [BitBuilt tag ds-lite](https://bitbuilt.net/forums/tags/ds-lite/)
- Raspberry Pi in DS Lite shell builds: Sudomod threads describe removing the cartridge slots, power parts and CPU and test-fitting a Pi in the bottom half. They also say the DS Lite was picked because its shell is thicker than the DSi or 3DS, and that routing wires through the hinge is the hardest part. Sudomod refused connections in this session, so these claims come from search snippets only. [Sudomod DS Lite Pi Zero (Understar)](https://sudomod.com/forum/viewtopic.php?p=54750), [Sudomod DS Lite Pi Zero 2016](https://www.sudomod.com/forum/viewtopic.php?p=12889)
- Pi Tin (CERN-OHL-W-2.0) and jusbot's caps (CC0, made for a mintyPi build) are custom handhelds that take genuine DS Lite caps. [Pi Tin](https://github.com/jackw01/pi-tin), [Printables 1178557](https://www.printables.com/model/1178557)

### Inferences
- The DSLITE_Remastered `.brd` is almost certainly gorgyrip's USG-CPU-10 Eagle file: same board, Eagle format, posted after gorgyrip's release. It is the only machine-readable outline of the genuine board found. With no licence from either party, use it for measurement only.
- Board-level projects (gorgyrip, nataliethenerd, DSLITE_Remastered) describe the internal envelope better than any shell model, because the board is a genuine part scanned flat.

### Gaps
- Whether gorgyrip's Dropbox set includes a 3D board model or component heights was not checked.
- No project was found that published a dimensioned drawing of the shell interior (wall thickness, boss heights, rib map). The Wesk scan is the closest.

## What sources pin down the internal envelope: board outline, lower LCD and touch module, battery?

### Takeaway
The board outline is solid: 124.26 x 69.22 mm from the scanned USG-CPU-10 file, with every hole and button position. The lower LCD is defined only by Nintendo's spec (3 inch, 256 x 192, 0.24 mm pitch); no panel part number or module drawing was found. The battery (USG-003) has no official dimensions; third-party clones list about 48 to 49 x 32 to 36 x 6 to 7 mm.

### Cited Findings
- **Board.** Measured here from `C-USG-CPU-10.brd`, layer 20 (Dimension, 82 segments):
  - Outline 124.26 x 69.22 mm (x 0.09 to 124.35, y 0.76 to 69.98).
  - The file is drawn from the CPU side, so it is mirrored relative to the face. x = 0 is the ABXY end (the console's right side seen from the front); y = 0 is the front edge, and the hinge edge is at y = 70.

  [C-USG-CPU-10.brd](https://raw.githubusercontent.com/DeaDDFuze/DSLITE_Remastered/main/pcb/original/C-USG-CPU-10.brd)
- Holes in the board file (x, y, drill, mm):
  - 4.0 mm: (10.21, 57.40), (27.18, 66.48), (2.84, 5.48), (97.13, 66.48), (121.43, 5.48).
  - 6.5 mm: (120.05, 52.48).
  - 1.8 mm: (22.24, 27.94), (110.08, 22.39).
  - 1.4 mm: (46.38, 66.21), (77.95, 66.21).

  [C-USG-CPU-10.brd](https://raw.githubusercontent.com/DeaDDFuze/DSLITE_Remastered/main/pcb/original/C-USG-CPU-10.brd)
- Element origins in the board file (x, y, mm; these are footprint origins, which may not be exact contact centres):
  - Buttons: A (3.70, 41.30), Y (18.84, 41.30), X (11.27, 48.95), B (11.27, 33.73).
  - D-pad: UP (113.66, 46.05), DOWN (113.20, 31.20), RIGHT (105.93, 38.86), LEFT (120.84, 38.39).
  - START (20.76, 16.88), SELECT (20.76, 9.19).
  - Shoulders: R (4.26, 60.49), L (120.13, 60.51).
  - Power switch SW13 (1.48, 49.28), headphones (9.98, 3.73), charge port (88.97, 66.80), battery connector (84.62, 44.27).
  - DS slot (63.46, 36.99), GBA slot (59.73, 20.00, rotated 270).
  - Bottom screen connector (115.65, 18.43), top screen and speaker connector (120.31, 59.82).
  - Mic (14.57, 61.89), Wi-Fi module DWM-W006 (20.08, 54.37).

  [C-USG-CPU-10.brd](https://raw.githubusercontent.com/DeaDDFuze/DSLITE_Remastered/main/pcb/original/C-USG-CPU-10.brd)
- The board is 6-layer. [nataliethenerd README](https://github.com/nataliethenerd/DSLite_ReverseEngineer)
- **Lower screen.** 3 inch diagonal, 256 x 192, 0.24 mm dot pitch, transmissive TFT with a resistive analogue touch screen. [nintendo.co.jp/ds/spec](https://www.nintendo.co.jp/ds/spec/index.html)
- Replacement lower LCDs are sold with the digitizer already bonded. Some are aftermarket with a different (matte) texture. No listing gave a panel part number or module outline. [eBay listing](https://www.ebay.com/itm/Replacement-Touch-Digitizer-Bottom-LCD-Display-Screen-Nintendo-DS-Lite-NDSL-/304548265937)
- **Battery.** The manual names the rechargeable battery USG-003. [Nintendo support a_id 16905](https://en-americas-support.nintendo.com/app/answers/detail/a_id/16905)
- Third-party USG-003 clones list varying sizes: 48.5 x 33 x 6 mm; 48.71 x 36.33 x 6.24 mm (850 mAh); 49.12 x 31.94 x 6.61 mm (2000 mAh); 48 x 34 x 7 mm. One seller warns that cells of 1000 mAh and up can be thicker and protrude. [Kaufland](https://www.kaufland.de/product/22858541/), [Newegg](https://www.newegg.com/centenex-electronics-banintendo-dslite-usg003-battery/p/0P2-001K-00002), [subtel](https://www.subtel.nl/en/Battery-for-Nintendo-DS-Lite-USG-001-USG-003-900mAh-Spare-Battery-Replacement-933066.html)
- **Battery cover.** The scanned aftermarket cover is 37.04 x 57.19 x 3.03 mm (measured here). [download](https://bitbuilt.net/wesk/3D%20Scans/DS%20Lite.rar)

### Inferences
- The board's 124.26 mm against the shell's 133.0 mm leaves 4.37 mm per side. Its 69.22 mm against the lower half's about 73.5 mm (patent) leaves about 2.0 mm at the front and 2.2 mm at the hinge. These are the wall-plus-clearance allowances the genuine shell uses at board level.
- Two 4.0 mm holes plus the 6.5 mm hole are candidates for the two board screws and the battery-bay boss. The 1.8 mm holes look like locating pins. This is unverified; match it against the FCC internal photos or the scan.

### Gaps
- No lower LCD panel or touch-panel part number, outline drawing or thickness was found.
- No genuine USG-003 dimensions or capacity from Nintendo or a teardown were found.
- Component heights (which set the board's keep-outs under the face) are not in the board file.

## Which numbers can be extracted now, and do the independent sources agree?

### Takeaway
The patent's top view (FIG. 3), scaled to 133.0 mm wide, and the scanned board file agree within about 0.5 mm on almost every bottom-half layout dimension: ABXY spacing, START and SELECT spacing, button distances from the edges. That makes the pair a trustworthy layout reference that needs no genuine unit. Outline numbers from the CAD models simply restate Nintendo's 133.0 mm.

### Cited Findings
- Method: FIG. 3 of D554194 was rendered at 80 dpi and inked lines were measured along scanlines. The outer width is 1800 px, set to Nintendo's 133.0 mm (13.53 px/mm). Distances are from the lower half's outer edges: the "back edge" is the back of the hinge barrel and the "front edge" is the near edge. [D554194 PDF](https://patentimages.storage.googleapis.com/46/a7/2c/bfb99f34860929/USD554194S1.pdf), [nintendo.co.jp/ds/spec](https://www.nintendo.co.jp/ds/spec/index.html)
- Results from patent FIG. 3, measured here:

  | Feature | Patent FIG. 3 (mm) | Cross-check |
  |---|---|---|
  | Lower half depth, back of hinge barrel to front edge | 73.5 | Nintendo 73.9 |
  | Plan aspect (FIG. 10, closed) | 1.788 | 133.0 / 73.9 = 1.800 |
  | D-pad opening (outer line) | 18.9 x 19.0 | DSLITE.md 18.9 |
  | D-pad cap outline (inner line) | 17.4 x 17.6 | DSLITE.md 17.5 to 17.8 |
  | D-pad centre from left edge | 15.0 | board: 10.96 + 4.37 = 15.3 |
  | D-pad centre from front edge | 40.8 | board: 37.85 + 2.0 = 39.9 (about 1 mm off) |
  | ABXY drawn circle diameter | 7.4 to 7.5 | DSLITE.md face hole 7.6 |
  | Y to A centres; X to B centres | 15.3; 15.2 | board 15.14; 15.21; gerbers 15.0 |
  | ABXY cluster centre from right edge | 15.6 | board: 11.18 + 4.37 = 15.55 |
  | ABXY cluster centre from front edge | 42.6 | board: 40.58 + 2.0 = 42.6 |
  | ABXY cluster centre from back edge | 30.8 | board: 28.64 + 2.2 = 30.8 |
  | START and SELECT circles | 3.6 dia, 7.7 apart | board 7.69 apart |
  | START and SELECT from right edge | 25.5 | board: 20.67 + 4.37 = 25.0 |
  | START, SELECT from front edge | 18.0, 10.4 | board 16.12, 8.43 (+2.0 = 18.1, 10.4) |
  | Lower screen window (inner line) | 62.8 x 47.3, centred, top 15.0 below back edge | active area 61.44 x 46.08 |
  | Screen recess (outer line) | about 70.5 wide | none |
  | Hinge barrel depth in plan | 8.4 to 8.8 | none |
  | Side edge double line (chamfer or parting line) | about 0.8 | none |

  [D554194 PDF](https://patentimages.storage.googleapis.com/46/a7/2c/bfb99f34860929/USD554194S1.pdf), [C-USG-CPU-10.brd](https://raw.githubusercontent.com/DeaDDFuze/DSLITE_Remastered/main/pcb/original/C-USG-CPU-10.brd)
- Outline from models, measured here:

  | Model | Width x depth x height (mm) | Licence | Provenance |
  |---|---|---|---|
  | Nintendo spec (closed console) | 133.0 x 73.9 x 21.5 | official | genuine |
  | Wesk scan, Bottom Cover (back half) | 133.29 x 74.24 x 13.78 | none stated | scan of aftermarket |
  | Wesk scan, Button Cover (face, with hinge knuckles) | 133.22 x 73.46 x 16.48 | none stated | scan of aftermarket |
  | facelesstech DS Lite base (back shell) | 133.0 x 73.95 x 13.5 | CC BY-NC-SA | not stated |
  | Macro F back / front | 133.0 x 73.67 x 12.6 / 9.0 | CC BY-NC | not stated |
  | gb9k GameboyMacro faceplate | 133.0 x 73.15 x 8.2 | MIT | not stated |
  | TimX24968B full console | 130.6 x 73.4 x 22.4 | CC BY-SA | tape measure |

  [download](https://bitbuilt.net/wesk/3D%20Scans/DS%20Lite.rar), [Printables 205873](https://www.printables.com/model/205873), [Printables 359015](https://www.printables.com/model/359015-macro-f/files), [gb9k/GameboyMacro](https://github.com/gb9k/GameboyMacro), [Printables 851425](https://www.printables.com/model/851425-nintendo-ds-lite-white)
- From the patent side view FIG. 7 (lid open), the lower half's thickness is about 14.7 to 15.4 mm depending on which plan dimension sets the scale. This is low confidence because side views are not drawn to the plan scale. [D554194 PDF](https://patentimages.storage.googleapis.com/46/a7/2c/bfb99f34860929/USD554194S1.pdf)

### Inferences
- Two independent records, Nintendo's own design drawing and a flatbed scan of a genuine board, agree within about 0.5 mm on the right-hand cluster and START and SELECT, and within about 1 mm on the D-pad's front-to-back position. So for the bottom half's top-view layout, FIG. 3 scaled to 133.0 mm is trustworthy enough to place openings, and the board file gives exact contact positions.
- The board sits about 4.4 mm in from each side wall and about 2 mm from the front and hinge-side walls.
- The drawn ABXY circle (7.4 to 7.5 mm) falls between the cap body (7.0) and DSLITE.md's 7.6 mm face hole. So it probably shows the hole edge with the cap inside it, at drawing line resolution.
- Thicknesses (face plate, back shell, lower-half height) cannot come from the patent. Take them from the Wesk scan (aftermarket) and confirm with calipers on a genuine shell.

### Gaps
- The scale of FIG. 3 rests on the drawn width equalling Nintendo's 133.0 mm. A 1% error in that assumption moves every number by about 1%.
- Corner radii, the face's edge chamfer profile and draft angles were not extracted. The patent shows them only qualitatively; the scan can give them.
- The D-pad front-to-back position differs by about 1 mm between the patent and the board footprint origin. It is unresolved whether the footprint origin is the contact centre.
