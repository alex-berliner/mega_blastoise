# DS Lite (USG-001) shell: materials, surface finish, colours, manufacturing, and how to approximate it on 3D prints

Scope: Nintendo DS Lite, model USG-001, with the bottom half as the focus. The original DS (NTR-001), DSi (TWL-001) and DSi XL are excluded; any source that mixes them up is flagged.

Confidence labels used on every finding:
- OFFICIAL: Nintendo documents (manual, press release, Nintendo Online Magazine developer interview, FCC filing by Nintendo).
- OBSERVED: seen in teardown or regulatory photos, or described first hand by press reviewers holding retail units.
- COMMUNITY: forum posts, buyer reviews, seller or vendor claims, blogs.
- ESTIMATED: inferred or typical-value figures, not measured on a DS Lite.

"(snippet)" means the fact came from a search-engine extract and the page itself was not opened (blocked or not fetched). Treat these as weaker.

Local copies of downloaded material (untrusted data, not in the repo) are under `/tmp/claude-1000/-home-alex-Code-mega-blastoise/5d435370-92de-4303-8bf6-69f783ed9bab/scratchpad/dslite/materials/`: `jp_manual/` (JP manual PDF plus text), `fcc/` (FCC label and external photos), `nom/` (archived Nintendo Online Magazine pages), `nintendo_pr/`, `wayback*/` (archived GBAtemp threads), `spraymax/`.

## 1. What plastic is the DS Lite made of (shell, buttons, lens, shoulder buttons, hinge, battery cover)? Are resin codes moulded inside?

### Takeaway
Nintendo never published the material of the USG-001 housing itself. Its Japanese manual does give the GBA slot dust cover (USG-005) as "ABS, PC/ABS". A 2006 reviewer said that cover is the same plastic as the outer shell, so the shell is most likely an ABS-family or PC/ABS-family plastic. No teardown, forum post or photo found in this research shows a moulded resin code (">ABS<", ">PC<", ">PC+ABS<") on a genuine shell, button, lens, hinge or battery cover. Claims online of "polycarbonate" or "1.2 mm ABS" come from unsourced SEO pages.

### Cited Findings
- **GBA connector cover USG-005.** The Japanese DS Lite manual's spec table gives the material as "プラスチック部（ABS、PC/ABS）" (plastic parts: ABS, PC/ABS). Size is 縦23.2 x 横57.0 x 厚さ8.5 mm, mass about 7.1 g. Source: [Nintendo JP DS Lite manual, spec page (printed p.72 to 73, PDF p.37)](https://www.nintendo.co.jp/ds/support/pdf/dsLitemanual.pdf). OFFICIAL.
- **Stylus USG-004.** Same table: the pen body is ABS and the tip is PE. Length about 87.5 mm, mass about 1.6 g. Source: [same manual](https://www.nintendo.co.jp/ds/support/pdf/dsLitemanual.pdf). OFFICIAL.
- **Strap NTR-009(-01).** Same table: the plastic part is ABS and the cord is PA nylon. Source: [same manual](https://www.nintendo.co.jp/ds/support/pdf/dsLitemanual.pdf). OFFICIAL.
- **Main unit USG-001.** The spec entry gives only size (縦73.9 x 横133.0 x 厚さ21.5 mm folded) and mass (about 218 g including battery pack, stylus and GBA cover). It gives no material. Source: [same manual, PDF p.36](https://www.nintendo.co.jp/ds/support/pdf/dsLitemanual.pdf). The size and weight match the launch press release: [Nintendo news release 2006-01-26](https://www.nintendo.co.jp/corporate/release/2006/060126.html). OFFICIAL.
- **Dust cover matches the shell.** Nintendo World Report on the GBA slot dust cover: "It's made of the same plastic as the outside of the DSL", so it "blends in perfectly". Source: [NWR DS Lite review, Jonathan Metts, 2006-04-27](https://www.nintendoworldreport.com/review/11365). OBSERVED (press review of an import unit).
- **Nintendo chose the materials itself.** In Nintendo's own developer interview, mechanical designer 米山 (Yoneyama) says his section selects "各パートで使う部品や材料" (the parts and materials used for each part). It also builds the design in 3D CAD with the product designer. No material is named. Source: [Nintendo Online Magazine No.93, April 2006, "ニンテンドーDS Lite開発スタッフインタビュー"](http://www.nintendo.co.jp/nom/0604/42/index.html) ([archived copy](https://web.archive.org/web/2006/http://www.nintendo.co.jp/nom/0604/42/index.html)). OFFICIAL.
- **Buttons use silicone contacts.** The DS Lite's buttons use silicone contacts (シリコン接点), so they lack the DS's click. Source: [Japanese Wikipedia, ニンテンドーDS Lite](https://ja.wikipedia.org/wiki/ニンテンドーDS_Lite). COMMUNITY (encyclopedia, consistent with teardown guides).
- **Conflicting third-party material claims (both unsourced):**
  - "OEM shells used about 1.2 mm ABS": [Alibaba buying guide](https://electronics.alibaba.com/buyingguides/nintendo-ds-lite-shell-replacement-guide) (snippet).
  - ABS "has higher thermal expansion than the original Nintendo polycarbonate blend": [Alibaba Q&A guide](https://electronics.alibaba.com/question/ds-lite-shell-replacement-guide-fit,-quality-diy-tips) (snippet).
  - COMMUNITY, low quality: these read as SEO text and contradict each other.
- **"Triple-layered polycarbonate shell".** Claimed by [Decked Out Magazine, "THE FILES 012: Nintendo DS Lite"](https://deckedoutmagazine.com/2026/03/02/the-files-012-nintendo-ds-lite/) (snippet; the page returned 404 when opened). FLAG: per the search extract, the same article gives the model as NTR-001, which is the original DS, so it conflates models. COMMUNITY, unreliable.
- **Aftermarket shells are sold as ABS.**
  - Hand Held Legend: each DS Lite shell "made from high-quality ABS plastic" ([HHL DS Lite shells](https://handheldlegend.com/collections/nintendo-ds-lite-shells), snippet).
  - eXtremeRate: "premium ABS material" ([HHL eXtremeRate listing](https://handheldlegend.com/products/full-shell-replacement-for-nintendo-ds-lite-extremerate)).
  - COMMUNITY (vendor). These describe replicas, not genuine parts.
- **White shoulder buttons yellow.** On white DS Lites, two GBAtemp users report the L and R buttons going "almost yellow" after about a year of ownership (Lee79 and assassinz, 2007-10-31). Source: [GBAtemp, "Gunk stunk inside clear plastic (white NDS Lite)"](https://gbatemp.net/threads/gunk-stunk-inside-clear-plastic-white-nds-lite.64176/). COMMUNITY.
- **Top screen lens:** no material source found for the DS Lite top-screen window. The bottom half has no separate lens; the touch panel sits in the face plate opening (iFixit treats the digitizer as its own part). Source: [iFixit, Nintendo DS Lite Disassembly, steps 21 and 27](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Disassembly/86279). OBSERVED.

### Inferences
- **Shell material.** The manual lists ABS and PC/ABS for the cover, and a reviewer says that cover matches the outer shell. So the outer shell is most likely ABS and/or PC/ABS, not straight PC. This is moderate confidence: Nintendo never states the shell material directly.
- **Two materials in one 7 g cover.** One plausible reading: it is built like the shell, a two-shot moulding with a clear layer and a coloured layer (see section 2). That would make one shot a transparent ABS-family grade and the other a PC/ABS, or the reverse. This is speculation; no source assigns the materials to layers.
- **Yellowing.** White L/R buttons yellowing within about a year fits a styrenic plastic (ABS-type) exposed to light. This is consistent with ABS but does not prove it.

### Gaps
- No photo or text found of a moulded resin identification mark inside a genuine DS Lite shell, face plate, hinge cover, battery cover, button cap, shoulder button or lens. iFixit and forum photos may show such marks, but nothing in this research quoted or captioned one. Checking a genuine shell by eye is the only reliable route.
- Material of the A/B/X/Y and D-pad caps, the L/R buttons, the hinge parts and the battery cover: not found.
- Top-screen lens material (PMMA vs PC): not found.

## 2. How was the high-gloss finish produced, which faces are gloss vs matte, and is there a texture spec?

### Takeaway
Nintendo's own developer interview states that the DS Lite's distinctive look comes from 二色成形 (two-colour, i.e. two-shot, moulding):
- Crystal White and Ice Blue use a transparent part.
- Enamel Navy uses a translucent, slightly blue-tinted part.
- The prototypes were so specialised that colour-check samples were made by an outside two-shot moulding specialist rather than in-house.

The outside (lid, back of the bottom half, and per photos the side band) is a glass-like gloss, "as if covered with a glass cover". The face around the D-pad and A/B/X/Y is deliberately matte (つや消し), and Nintendo pitched that contrast as the premium cue. The manual also warns that solvents can make the "本体の塗装" (the body's paint or coating) peel, so some painted layer is present too. No VDI or MT texture code was found.

### Cited Findings
- **Two-shot moulding (Nintendo's statement).** Sakiyama (崎山), mechanical design: "今回は２色成形で独特な質感を出しています。クリスタルホワイトとアイスブルーに関しては透明な部品を、エナメルネイビーには薄いブルーがかった半透明の部品が使われています。本来はカラーチェック用の試作品は社内で作ってしまうのですが、今回は２色成形のために外部の専門業者さんに依頼しています。" Translation: "This time we created a distinctive texture with two-colour moulding. Crystal White and Ice Blue use a transparent part; Enamel Navy uses a translucent part with a light blue tint. Normally we make colour-check prototypes in-house, but this time, because of the two-colour moulding, we commissioned an outside specialist." Source: [NOM No.93, April 2006, developer interview](http://www.nintendo.co.jp/nom/0604/42/index.html) ([archive](https://web.archive.org/web/2006/http://www.nintendo.co.jp/nom/0604/42/index.html)). OFFICIAL.
- **Gloss outside, matte face (Nintendo's own magazine).** "本体の表面はガラスのカバーをかぶせたような光沢を持っています。それとは反対に十字ボタンやＡ・Ｂ・Ｘ・Ｙボタンの周囲はつや消し処理がしてあって、本体の内と外における２種類の質感の対比が独特の高級感を演出しています。" Translation: "The body surface has a gloss as if a glass cover had been placed over it. In contrast, the area around the D-pad and A/B/X/Y is given a matte treatment, and the contrast of the two textures, inside versus outside, creates a distinctive premium feel." Source: [NOM No.93, "ニンテンドーDS Liteとは"](http://www.nintendo.co.jp/nom/0604/41/index.html) ([archive](https://web.archive.org/web/2006/http://www.nintendo.co.jp/nom/0604/41/index.html)). OFFICIAL (Nintendo's in-house magazine, editorial voice).
- **Lid inner recess.** The same page says the inside of the lid is slightly recessed and works as a thumb grip when the unit is held vertically. Source: [NOM 4-1](http://www.nintendo.co.jp/nom/0604/41/index.html). OFFICIAL.
- **Clear layer seen on a pre-launch mockup.** PC Watch, 2006-02-16, disassembling a pre-launch mockup: "白に透明な層をまとったボディはiMacやiBookを思わせる" (the body, white wrapped in a transparent layer, recalls the iMac and iBook). Caption: "ボディは、白い樹脂の外側に透明な樹脂の層がある" (the body has a layer of transparent resin outside the white resin). "裏側から光を当てると透明な構造がよくわかる" (backlighting from behind shows the transparent structure clearly). PC Watch believed the exterior parts were the same as production. Source: [PC Watch, ニンテンドーDS Lite"モックアップ"分解レポート](https://pc.watch.impress.co.jp/docs/2006/0216/nintendo.htm). OBSERVED (teardown photos).
- **Users see a clear layer with a hairline gap.**
  - kentl901 (2007-10-31), white unit: "the clear plastic surrounds the white interior plastic", with "a very thin gap between the clear plastic and white plastic" where dirt collects. Heran Bago: "If you take your DS Lite apart the plastic comes off of the bottom." 4saken disagrees: "I don't think you can remove the plastic casing from the shell itself." Source: [GBAtemp thread 64176](https://gbatemp.net/threads/gunk-stunk-inside-clear-plastic-white-nds-lite.64176/).
  - kobykaan (2009-08-12): the "hard plastic coating that's on the out side of the ds... you cannot separate the two without forcing it... they are glued in there". raskal549: four Coral Pinks and two Whites all collected dirt in "the skinny area of clear plastic", which can only be cleaned by full disassembly. Source: [GBAtemp thread 173506](https://gbatemp.net/threads/cleaning-nintendo-ds-lite.173506/).
  - COMMUNITY.
- **Matte white case under a clear cover.** "the matte white case and the clear cover" (DownwardSpyral, 2007-01-30). An Onyx (black) owner wants to "remove the clear shell and paint the stuff under it". Source: [GBAtemp, "Painting a DS Lite"](https://gbatemp.net/threads/painting-a-ds-lite.43492/). COMMUNITY.
- **Japanese owner describes the same structure.** 2009-11-20: "クリアな外装と、白い内装（本体）の間の埃" (dust between the clear exterior and the white interior body). Nintendo quoted about 3,500 yen to clean it at the repair centre. Source: [Yahoo!知恵袋 q1333204243](https://detail.chiebukuro.yahoo.co.jp/qa/question_detail/q1333204243). COMMUNITY.
- **Index card in the seam.** Sliding an index card "between the plastic outer shell and the inner part" (viewtiful_dru, 2010). Source: [narkive, "how to clean a DS lite?"](https://games.electronics.narkive.com/SCmjbgK1/how-to-clean-a-ds-lite). COMMUNITY.
- **Painted coating (official).** The manual tells owners not to wipe with thinner, benzine, other volatile oils or alcohol: "プラスチックを傷める原因となったり、本体の塗装がはがれることがあります" (it can damage the plastic or make the body's paint/coating peel). It recommends a cloth dampened with diluted neutral detergent, then a dry cloth. Source: [Nintendo JP DS Lite manual, PDF p.12, 使用上のおねがい](https://www.nintendo.co.jp/ds/support/pdf/dsLitemanual.pdf). OFFICIAL.
- **Painted special edition.** The Pokémon Center Dialga/Palkia Edition (2006-09-28) is described as using metallic black paint. The Club Nintendo Premium DS Lite (Premium Silver, Premium Black, Premium Rose, 1,000 units each) had a "鏡面風のつやがある素材" (mirror-like glossy material) on the top lid. Source: [Japanese Wikipedia](https://ja.wikipedia.org/wiki/ニンテンドーDS_Lite). COMMUNITY (encyclopedia).
- **Reviewers on gloss vs matte.**
  - NWR: "The new shell is very shiny and flat, and like the PSP and iPod, it picks up fingerprints like a magnet." "the attractive outer shell does not extend to the inside surfaces." "The matte finish inside is even off-tone, so the colors don't match." "the edges where the system folds together appear to be yet another, lighter color." "All three colors have different outside and inside tones." Source: [NWR review, 2006-04-27](https://www.nintendoworldreport.com/review/11365).
  - Shacknews, Enamel Navy, 2006-03-27: outside is "smooth, reflective plastic" similar to the PSP; "The plastic used on the inside of DS Lite has a matte finish", similar to the bottom of the original DS. Source: [Shacknews hands-on](https://www.shacknews.com/article/41445/nintendo-ds-lite-enamel-navy).
  - CNET, 2006: "high-gloss finish is a magnet for fingerprints, especially the darker-colored models". Source: [CNET review, mirrored by RetroIsle](https://www.retroisle.com/others/nintendods/Articles/dslite_review_cnet.php).
  - OBSERVED.
- **Owner preference for the inner texture.** A NeoGAF owner sanded the gloss off because they preferred "a nice textured feel in my hands like the inner casing over a slippery smooth plastic". Source: [bit-tech, 2006-09-27](https://bit-tech.net/news/modding/Scratched_up_DS_Lite_Mod/1). Gigazine says the result looked like an ordinary scratched unit: [Gigazine, 2006-09-27](https://gigazine.net/gsc_news/en/20060927_ds_lite) (snippet). COMMUNITY.
- **Polishing advice.** Pocket Gamer distinguishes the "shiny outer areas" from "matt plastic sections". It suggests Brasso on the gloss only ("rubbing away the top layer"), no more than once or twice in the console's life. Source: [Pocket Gamer, 2006-08-23](https://www.pocketgamer.com/how-tos/how-to-remove-ds-lite-scratches). COMMUNITY.
- **Gloss on the lid, back, battery cover and side band (FCC photos).** On a white pre-production unit photographed for the FCC (grant 2006-02-03), the lid, the bottom half's back, the battery cover and the side band all show specular highlights, while the face around the touch screen reads as satin. Sources: [FCC ID BKEUSG001 exhibits](https://fccid.io/BKEUSG001), [external photos](https://fccid.io/BKEUSG-001/External-Photos/External-Photos-626995). OBSERVED (regulatory photos).

### Inferences
- **What the finish is.** The DS Lite's "piano" gloss is not just a polished single-colour moulding. It is a two-shot part: a clear or translucent outer skin over a coloured (white, blue, navy, and per owners also black and pink) body. This explains three observations:
  - the "glass cover" depth;
  - why scratches and dirt sit in or under a clear layer;
  - why NWR saw the fold edges as "yet another, lighter color" (light piping through the clear layer's edge).
- **The hairline gap.** The thin gap where dirt collects suggests the two shots are not fully fused at the perimeter. Alternatively, the clear skin may wrap the edge and leave a seam. Owners disagree on whether it can be separated, and two-shot moulding normally gives one inseparable part.
- **Paint and two-shot are not exclusive.** The manual's 塗装 warning may be generic Nintendo wording, but it suggests some surfaces or colours are painted or coated as well. Painted candidates: the metallic Gloss Silver, Metallic Rose and Dialga/Palkia units, or printed or coated layers on the coloured shot.
- **Which faces are which.** The gloss covers the "outside" faces: lid, back of the bottom half, battery cover, and probably the side band. The face plate around the D-pad, A/B/X/Y and screen is matte. For a bottom-half replica, that means a matte top face and a gloss back and sides.

### Gaps
- **Unknown process details:**
  - whether the clear shot is moulded first (core-back or rotary two-shot) or over-moulded onto the coloured shot;
  - how thick the clear skin is;
  - whether the coloured shot carries paint;
  - whether there is any UV or hard coat on top.
- **Later colours.** Whether all later colours (Jet Black or Onyx, Noble Pink or Coral Pink, Metallic Rose, Gloss Silver, Crimson/Black, Cobalt/Black) also use two-shot. Owner reports suggest clear layers on Onyx and Coral Pink, but nothing official was found.
- **Texture specification.** No VDI 3400 or MoldTech (MT) code was found for the matte face, and no gloss-unit spec for the gloss faces.
- **Japanese trade press.** No Nikkei Electronics or other trade-press teardown discussing the housing process was found. PC Watch's mockup report is the closest.

## 3. Wall thickness, rib and boss structure, and mould tooling (gates, ejector pins, parting lines)

### Takeaway
No measured wall thickness for a genuine DS Lite shell was found:
- The project's earlier note gives a 1.4 to 1.5 mm face plate, from a 3D scan of an aftermarket shell.
- An unsourced seller page says "about 1.2 mm ABS".

Nintendo describes a deliberately thin lid and reinforced internal structure, backed by long strength testing. A pre-launch teardown shows a separate reinforcing member screwed to the bottom. No gate, ejector-pin or parting-line documentation was found.

### Cited Findings
- **Thin lid, custom LCD.** Sakiyama: the lid "外観はＤＳと同じ厚さに見えるが、内側から見るとかなり薄い" (looks as thick as the DS from outside but is quite thin seen from inside). Because the lid takes impacts, Nintendo had a custom, stronger and thinner LCD module made. Source: [NOM No.93 interview](http://www.nintendo.co.jp/nom/0604/42/index.html). OFFICIAL.
- **Strength work.** Yoneyama: "内部構造の補強や部品の配置バランスも含めて強度をかなり上げ、強度試験は長期間かけて行っています" (strength was raised considerably, including reinforcing the internal structure and balancing the part layout, and strength tests were run over a long period). He describes the trade-off against weight and size. Source: [same interview](http://www.nintendo.co.jp/nom/0604/42/index.html). OFFICIAL.
- **Reinforcement and captive screw (mockup).** "ボディの底部には補強材と思われる部材が3本ものネジで止められている" (a member that appears to be a reinforcement is held to the bottom of the body by three screws). The battery-cover screw is captive, as on the DS. Source: [PC Watch mockup teardown, 2006-02-16](https://pc.watch.impress.co.jp/docs/2006/0216/nintendo.htm). OBSERVED.
- **Fasteners from the back (iFixit):**
  - PH000 screws: two gold under the rubber feet and two silver in the battery compartment.
  - Y00 tri-point screws: three silver and one small hidden black.
  - Inside: three gold screws on the stylus retainer and two gold on a metal shield; a "back metal cover" that should not be removed.
  - The guide calls the latch the "chief design flaw". FLAG: the DS Lite has no lid latch, so the word "latch" is ambiguous here.
  - Source: [iFixit, Nintendo DS Lite Disassembly](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Disassembly/86279). OBSERVED.
- **Back layout (FCC photo).** Four corner screw holes, a battery cover on the right with its own screw, and a stylus silo on the left edge. Source: [FCC label and location exhibit](https://fccid.io/BKEUSG-001/Label/FCC-ID-Label-626994). OBSERVED.
- **Face plate measurements from the project's own notes.** Face plate 1.4 to 1.5 mm; button guide tubes ending about 3.0 mm under the face; board top about 3.0 to 3.4 mm under the face; seven screws into face bosses (two through the board); face nested in a stepped wall. This came from a 3D scan of an aftermarket shell (Wesk on BitBuilt) plus a reverse-engineered board. Source: `/home/alex/Code/mega_blastoise_dslite/hardware/ref_design/case/DSLITE.md`. COMMUNITY (aftermarket part, not genuine).
- **"About 1.2 mm ABS".** Claimed for OEM shells, unsourced: [Alibaba guide](https://electronics.alibaba.com/buyingguides/nintendo-ds-lite-shell-replacement-guide) (snippet). COMMUNITY, low quality.
- **General moulding guidance.** Protolabs gives typical injection-moulded wall ranges: ABS 0.045 to 0.140 in (1.14 to 3.56 mm) and PC 0.040 to 0.150 in (1.02 to 3.81 mm). Source: [Protolabs, solving wall thickness issues](https://www.protolabs.com/resources/design-tips/solving-wall-thickness-issues-in-molded-parts/) (snippet). ESTIMATED (general guidance, not the DS Lite).
- **Aftermarket screw posts.** eXtremeRate posts are not pre-threaded, so screws self-tap, and over-tightening "can crack the shell". Source: [HHL eXtremeRate listing](https://handheldlegend.com/products/full-shell-replacement-for-nintendo-ds-lite-extremerate). COMMUNITY (vendor).

### Inferences
- **Likely thickness.** A genuine shell wall in the 1.2 to 1.5 mm range is plausible: it matches both weak data points and sits at the thin end of the Protolabs ABS range, as expected for a 21.5 mm folded handheld. The two-shot construction means that thickness is split between a clear skin and a coloured body.
- **Screws.** Self-tapping screws into plastic bosses (PH000 and Y00 into moulded posts) are the norm here, which matters for boss design in a printed replica.

### Gaps
- Measured wall thickness of genuine bottom-half parts (face plate, back shell, side band), and the clear-skin thickness: not found.
- Rib layout, boss outer and inner diameters, draft angles, gate locations, ejector-pin marks and parting lines: none documented in any source found. Teardown photos (iFixit, PC Watch) could be examined by eye, but no captioned description exists.

## 4. Durability and failure modes (gloss scratching, hinge cracks, yellowing, boss cracks)

### Takeaway
The best-documented failure is the 2006 hinge crack: a near-identical crack on the left side of the hinge, which Nintendo of America called cosmetic, affecting under 0.02 percent of US units, and repaired free. The gloss collects fingerprints and fine scratches; reviewers disagree on how easily it scratches. Dirt gets trapped in the seam between the clear and coloured layers on light colours. White shoulder buttons yellow. No reports of genuine screw-boss cracking were found; it is reported for aftermarket shells.

### Cited Findings
- **First hinge-crack reports.** Joystiq reported them on 2006-07-17: owners show a near-identical crack on the left side of the hinge, mostly cosmetic, but some say it can worsen until the hinge casing is lost. Nintendo's first statement: "We are not aware of any problems with the hinge of the DS Lite", with $50 repairs quoted. Source: [Engadget, 2006-07-18](https://www.engadget.com/2006-07-18-nintendo-we-are-not-aware-of-any-problems-with-the-hinge-of-th.html). OBSERVED (press, owner photos).
- **Nintendo's position.** Nintendo of America said the cracks represent "less than 0.02 percent of the total units sold" in the US: "This cosmetic issue in no way impacts the gameplay or integrity of the DS Lite". It offered free repair. Source: [Gamasutra/Game Developer, 2006-07-21](https://www.gamedeveloper.com/pc/nintendo-agrees-to-fix-ds-lite-hinge-cracks). OFFICIAL (quoted statement).
- **Japanese coverage.** The Japanese Wikipedia article records hinge-part cracking ("ヒンジ（つなぎ目）のパーツが割れる") after the US launch, with NOA's 0.02 percent figure and free repair. It cites archived Engadget Japanese articles from 2006-07-17 and 2006-07-22. Source: [Japanese Wikipedia](https://ja.wikipedia.org/wiki/ニンテンドーDS_Lite). COMMUNITY.
- **Cause theories, all unconfirmed:**
  - hinge too tight, stressing the plastic ([Nintendo Life forum](https://www.nintendolife.com/forums/ds/ds_lite_hinge_replacement_question), snippet);
  - a seized hinge spring (Reddit repair thread, snippet);
  - "original Nintendo plastic is brittle near the hinges" ([GameFAQs](https://gamefaqs.gamespot.com/boards/925329-nintendo-ds/76520478), snippet, page blocked).
  - COMMUNITY.
- **Nintendo's own strength goal.** Strength testing was run over a long period, with the aim that a dropped unit should not cost owners repair money. Source: [NOM No.93](http://www.nintendo.co.jp/nom/0604/42/index.html). OFFICIAL.
- **Gloss wear.**
  - NWR: fingerprints accumulate within "half an hour"; but "the outer shell isn't as easy to scratch as you might think" ([NWR](https://www.nintendoworldreport.com/review/11365)).
  - CNET: darker models are "just as susceptible to scratches and smudges" as the PSP and iPod ([CNET via RetroIsle](https://www.retroisle.com/others/nintendods/Articles/dslite_review_cnet.php)).
  - OBSERVED. The two reviews disagree on scratch resistance.
- **Paint and coating damage.** Solvents (thinner, benzine, alcohol) may make the body's paint/coating peel ([JP manual](https://www.nintendo.co.jp/ds/support/pdf/dsLitemanual.pdf), OFFICIAL). A 2006 Crystal White owner reported black specks and a small paint loss near the buttons on "white painted parts". Other owners attributed it to skin oil and dirt, and one suggested compound polishing ([kakaku.com BBS, 2006-07-28 to 31](https://bbs.kakaku.com/bbs/J0000010032/SortID=5297560/), COMMUNITY; the page text was partly garbled).
- **Dirt in the clear/colour seam.** Reported on White, Polar White and Coral Pink; not visible on dark colours ([GBAtemp 64176](https://gbatemp.net/threads/gunk-stunk-inside-clear-plastic-white-nds-lite.64176/), [GBAtemp 173506](https://gbatemp.net/threads/cleaning-nintendo-ds-lite.173506/), [Yahoo!知恵袋](https://detail.chiebukuro.yahoo.co.jp/qa/question_detail/q1333204243)). COMMUNITY.
- **Yellowing.** White DS Lite L/R buttons "almost yellow" after a year ([GBAtemp 64176](https://gbatemp.net/threads/gunk-stunk-inside-clear-plastic-white-nds-lite.64176/)). COMMUNITY. FLAG: an AtariAge thread sometimes cited for "DS Lite Rose yellowing" is about yellowed screens, not shells ([AtariAge 303922](https://forums.atariage.com/topic/303922-nintendo-ds-yellow-screens/)).
- **Battery cover.** iFixit notes the battery cover "becomes sticky with age" and may need prying. Whether that means stuck or tacky is unclear. Source: [iFixit disassembly, step 2](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Disassembly/86279). OBSERVED.
- **Screw-boss cracking (aftermarket).** Reported for aftermarket shells: over-tightened self-tapping screws crack posts, and screws pushed through the surface of an eXtremeRate bottom half ([HHL eXtremeRate listing and reviews](https://handheldlegend.com/products/full-shell-replacement-for-nintendo-ds-lite-extremerate)). Knock-off shells "badly molded and their screws don't grip well" (masbass, 2009, [GBAtemp 173506](https://gbatemp.net/threads/cleaning-nintendo-ds-lite.173506/)). COMMUNITY.

### Inferences
- **Hinge stress.** The repeatable left-hinge crack points to a stress concentration at a thin hinge wall or knuckle under spring or friction load, not to a weak material overall. A printed replica should give the hinge region generous fillets and wall.
- **Two-shot trade-off.** A two-shot gloss skin brings two durability side effects: an edge seam that traps dirt, and scratches that sit in a clear layer (they can be polished out, which is consistent with the Brasso advice).

### Gaps
- No reports were found of genuine DS Lite screw bosses stress-cracking, or of the gloss skin delaminating.
- No yellowing reports were found for genuine white shell bodies (as opposed to shoulder buttons).
- No root-cause analysis of the hinge crack from Nintendo.

## 5. Colour lineup by region; moulded-in vs painted; Pantone or RAL approximations

### Takeaway
Japan launched Crystal White on 2006-03-02. Ice Blue and Enamel Navy slipped to 2006-03-11 after "unexpected trouble in the manufacturing process", in the same period Nintendo describes two-shot moulding with clear and translucent parts. Later colours and many limited editions followed. North America launched Polar White, then Onyx and Coral Pink. For at least the launch colours, colour is moulded-in under a clear or translucent shot. Some special editions are painted. No Pantone or RAL approximations were found.

### Cited Findings
- **Launch delay of Ice Blue and Enamel Navy.** Nintendo planned Crystal White, Ice Blue and Enamel Navy for 2006-03-02. "製造工程で予期せぬ支障が生じ" (unexpected trouble arose in the manufacturing process), so the two colours could not be secured in quantity at sufficient quality, and moved to 2006-03-11. Source: [Nintendo news release 2006-02-24](https://www.nintendo.co.jp/corporate/release/2006/060224.html). OFFICIAL. The reason is not specified further.
- **Colour concept.** Colourings were proposed by the designer and chosen to be easy for women and older people to pick, with a "LIGHT" image. Source: [NOM No.93](http://www.nintendo.co.jp/nom/0604/42/index.html). OFFICIAL. The same interview names the product designer as 江原唯 (Yui Ehara).
- **Japan standard colours.** Crystal White 2006-03-02; Ice Blue and Enamel Navy 2006-03-11 (Enamel Navy Japan-only, discontinued summer 2007); Noble Pink 2006-07-20; Jet Black 2006-09-02; Metallic Rose and Gloss Silver 2007-06-23; Crimson/Black 2007-10-04. Source: [Japanese Wikipedia](https://ja.wikipedia.org/wiki/ニンテンドーDS_Lite). COMMUNITY (encyclopedia). Metallic Rose and Gloss Silver also confirmed as new Japanese colours by [Gizmodo, 2007-06-25](https://gizmodo.com/hands-on-with-japans-ds-colors-271975).
- **Outside Japan:**
  - Cobalt/Black: North America and South Korea, 2008-02-10.
  - Passion Red and Champagne Gold: Hong Kong and Taiwan, March 2011.
  - Gold: North America, Zelda Phantom Hourglass bundle.
  - Source: [Japanese Wikipedia](https://ja.wikipedia.org/wiki/ニンテンドーDS_Lite). COMMUNITY.
- **North America.** Polar White at launch (2006-06-11); Coral Pink and Onyx Black from 2006-09-13 at $129.99. Source: [Game Developer, 2006-08-24](https://www.gamedeveloper.com/pc/black-pink-ds-lites-to-debut-in-north-america). OBSERVED (press). CNET lists "Polar White, Onyx (black)" ([CNET via RetroIsle](https://www.retroisle.com/others/nintendods/Articles/dslite_review_cnet.php)).
- **Europe:**
  - White and black at the 2006-06-23 launch ([Game Developer](https://www.gamedeveloper.com/game-platforms/nintendo-releases-ds-lite-in-europe), snippet).
  - Silver added on 12 October, joining black, white and pink ([GameSpot](https://www.gamespot.com/articles/silver-ds-lite-comes-to-europe/1100-6178674/), snippet; the year was not confirmed because the page was blocked).
  - Red, turquoise and green on 2008-06-13 at £99 ([Game Developer, 2008-05-21](https://gamedeveloper.com/game-platforms/new-ds-colors-i-cooking-guide-i-announced-for-europe)).
  - OBSERVED (press).
- **User-compiled regional list:**
  - Europe: White, Smart Black, Noble Pink, Platinum, Red, Turquoise, Green.
  - Australia: Polar White, Jet Black, Coral Pink, Rose Metal.
  - North America: Metallic Rose, Metallic Silver, Red/Black, Blue/Black.
  - Source: [Nintendo Life forum](https://www.nintendolife.com/forums/ds/what_ds_lite_and_dsi_colors_are_available_in_all_the_reigons) (snippet). COMMUNITY, low confidence.
- **Selected limited editions (Japan):**
  - Crystal Edition (FFIII, 2006-08-24).
  - Dialga/Palkia Edition (2006-09-28, metallic black paint).
  - Pikachu Edition (2007-07-20, custom yellow).
  - Wonderful World Edition (Gloss Silver).
  - BABY MILO Edition (Crystal White and gold).
  - Giratina Edition (2008-09-13).
  - Club Nintendo Premium Silver, Premium Black and Premium Rose (1,000 each, mirror-like lid).
  - Source: [Japanese Wikipedia](https://ja.wikipedia.org/wiki/ニンテンドーDS_Lite). COMMUNITY.
- **Moulded-in colour under clear (official).** Crystal White and Ice Blue use a transparent part; Enamel Navy a light-blue translucent part ([NOM No.93](http://www.nintendo.co.jp/nom/0604/42/index.html), OFFICIAL). A white resin body sits under a transparent resin layer ([PC Watch](https://pc.watch.impress.co.jp/docs/2006/0216/nintendo.htm), OBSERVED). Owners report the same clear-over-colour structure on White, Coral Pink and Onyx ([GBAtemp 173506](https://gbatemp.net/threads/cleaning-nintendo-ds-lite.173506/), [GBAtemp 43492](https://gbatemp.net/threads/painting-a-ds-lite.43492/), COMMUNITY).
- **Inside and outside tones differ.** NWR: "All three colors have different outside and inside tones." Source: [NWR](https://www.nintendoworldreport.com/review/11365). OBSERVED.
- **"Clear +" listing names.** A refurbisher lists variants as "Clear + Polar White", "Clear + Metallic Rose", "Clear + Gloss Silver" ([eBay.de listing](https://www.ebay.de/itm/203245745063), snippet). COMMUNITY. This suggests replacement-shell makers also sell clear-over-colour shells, but the listing does not explain the build.
- **Pantone or RAL.** No match for any DS Lite colour was found from modders, refinishers or aftermarket sellers (searches in English and Japanese). The JP manual PDF uses a "PANTONE Warm Gray 11 C" spot colour, but that is the manual's print ink, not a shell colour ([PDF metadata](https://www.nintendo.co.jp/ds/support/pdf/dsLitemanual.pdf)).

### Inferences
- **What the delay may have been.** The Ice Blue and Enamel Navy slip coincides with Nintendo's comments that those colours rely on clear and translucent two-shot parts made with an outside specialist. A yield problem in two-shot moulding of tinted translucent parts is plausible, but Nintendo never said so.
- **Matching a colour.** Each DS Lite colour is really two colours plus optics: a coloured inner shot seen through a clear or tinted skin, and a separate matte face colour that NWR noted does not match. A colour match needs a physical sample measured on both faces.

### Gaps
- Official colour specs (Munsell, Pantone, RAL) for any DS Lite colour: none found.
- Which later colours are painted vs moulded-in: not confirmed. Metallic Rose and Gloss Silver are the likely painted or metallic candidates.
- An English-language first-party colour list by region was not found. The regional lists above are compiled from press and an encyclopedia.

## 6. Markings on the bottom half (and the lid): printed, embossed or debossed?

### Takeaway
Nintendo's FCC exhibit (white pre-production unit, February 2006) shows the bottom half's markings as grey ink graphics on the gloss back and sides, with only the serial number on a separate sticker:
- the regulatory block, Nintendo and "NINTENDO DS" logos, CE and recycling marks;
- "SLOT-1 PUSH TO EJECT", "SLOT-2" (three times along the edge) and "EXT.";
- "POWER" on the right side and "VOL." on the left;
- "START" and "SELECT" beside ringed buttons on the face.

The lid of that sample carries a tone-on-tone "DS" double-rectangle mark rather than a "Nintendo DS lite" wordmark. No source states the print process; pad printing is the likely method but is unconfirmed.

### Cited Findings
- **Proposed FCC label and its location.** The proposed FCC ID label ("FCC ID: BKEUSG-001", Part 15 text) goes on the back of the bottom half, lower left. The photo of the back also shows:
  - grey-printed "SLOT-1 PUSH TO EJECT" with an eject icon, top centre;
  - "EXT." top right;
  - the Nintendo oval logo and a "NINTENDO DS" wordmark;
  - a small-text regulatory block in English and French (model USG-001, ©2006 Nintendo, patents, ratings, battery and adapter references, "MADE IN CHINA");
  - CE mark, recycling marks and a crossed-out wheelie bin;
  - "SLOT-2" printed three times along the bottom edge;
  - a separate serial-number sticker reading "USGPPF2A117".

  The battery cover (right) is the same glossy white with a screw at its lower right. Source: [FCC exhibit "FCC ID Label" (ID Label/Location Info)](https://fccid.io/BKEUSG-001/Label/FCC-ID-Label-626994), filing [BKEUSG001](https://fccid.io/BKEUSG001), grant 2006-02-03. OFFICIAL (Nintendo's filing) and OBSERVED (photo).
- **Other external photos (same filing).**
  - Face: "START" and "SELECT" in grey text to the right of two small round buttons, each in a circular ring.
  - Sides: "POWER" printed above the slide switch on the right side; "VOL." beside the slider on the left side.
  - Lid: a twin rounded-rectangle "DS" mark in the centre, tone-on-tone (white on white, outline style), plus a lab test sticker "O042-01".
  - Source: [FCC External Photos](https://fccid.io/BKEUSG-001/External-Photos/External-Photos-626995). OBSERVED. FLAG: this is a pre-production sample; retail markings may differ.
- **Stickers in teardowns.** iFixit mentions a sticker covering upper screws (step 19) and a sticker to remove "only when changing shell color" (step 26). Source: [iFixit DS Lite disassembly](https://www.ifixit.com/Guide/Nintendo+DS+Lite+Disassembly/86279). OBSERVED.
- **Serial number location (conflicting).** A Q&A site claims the serial label is under the battery cover ([answers.com Q&A](https://qa.answers.com/games-qa/Where_is_the_serial_number_on_a_Nintendo_ds_lite), snippet seen only through a mirror of this URL, COMMUNITY, weak). The FCC photo shows the serial sticker on the outside of the back, below the regulatory print (OBSERVED). These conflict; retail placement was not verified.

### Inferences
- **Print method.** The small grey legends on curved and glossy faces (side "POWER" and "VOL.", back block, face "START" and "SELECT") are typical of pad printing. Because the colour sits under a clear shot, the back print is probably on top of the clear skin, but this was not verified (ESTIMATED).
- **Lid mark.** The tone-on-tone "DS" lid mark looks like a feature of the moulding (raised, or printed on the inner shot) rather than surface ink. The photo resolution cannot settle this.

### Gaps
- No source states pad printing vs screen printing vs in-mould marking for any DS Lite legend.
- Whether the A/B/X/Y letters on genuine caps are printed, moulded or two-shot: not found.
- Retail lid logo variants by region ("DS" mark vs "Nintendo DS lite" wordmark): not verified in this research.
- Any embossed or debossed text inside the bottom shell (mould numbers, cavity numbers, resin codes): not found.

## 7. How aftermarket replacement shells differ from genuine ones

### Takeaway
Buyers and modders consistently describe aftermarket DS Lite shells as worse than genuine:
- thinner internal walls (speakers visible through light colours);
- fragile hinge pieces;
- poor moulding, with screws that strip or push through;
- warping in heat, and gaps.

Even the best-known brand (eXtremeRate) gets mixed reviews on material quality. Aftermarket shells are sold as plain ABS (opaque, clear-tinted, or "soft touch"), not as Nintendo's two-shot clear-over-colour construction.

### Cited Findings
- **Old replacement shells were thin and fragile.** Cosmic_Link, 2021-06-17: back when the DS Lite was current, replacement shells "tended to be pretty crap where the hinge pieces were concerned and broke quite easily", and "the internal shell pieces were quite thin... where the speakers were and could be seen through the plastic on light coloured shells". The advice was to keep the original internal pieces. Duo8: "Replica shells are pretty much always crap. At least they appear to have been made with (discarded) original molds." Source: [GBAtemp thread 590071](https://gbatemp.net/threads/seeking-suggestions-for-ds-lite-shell-parts.590071/). COMMUNITY.
- **Knock-offs badly moulded.** masbass, 2009-09-14: "those chinese knock-off shells they sell on ebay are badly molded and their screws don't grip well all of the time". After a shell swap, finger smudges still show "on the glossy outside". Source: [GBAtemp thread 173506](https://gbatemp.net/threads/cleaning-nintendo-ds-lite.173506/). COMMUNITY.
- **eXtremeRate kit (HHL listing):**
  - "premium ABS material"; "Soft Touch" is "a matte, soft texture, anti-slip finish"; also clear, chrome and chameleon variants;
  - no hinges included; screw posts not pre-threaded, and over-tightening can crack the shell.
  - Reviews: "material quality" falls short, and screws pushed through the bottom half's surface and scratched the hinge (Bishop L.); screws did not hold, original screws used, left bumper sat poorly (Darrell V.); slight warping of the bottom half's upper edge after shipping in heat (Lenny); positive comments on finish and plastic quality from others.
  - Source: [Hand Held Legend, eXtremeRate DS Lite full shell](https://handheldlegend.com/products/full-shell-replacement-for-nintendo-ds-lite-extremerate). COMMUNITY (vendor and buyers).
- **New eXtremeRate tooling.** A YouTube reviewer says eXtremeRate's DS Lite shell uses an entirely new mould. Source: ["A DS Shell That's Actually GOOD!?"](https://www.youtube.com/watch?v=0R1EHPq4dkc) (snippet). COMMUNITY.
- **eBay buyer reviews.** "not like OEM quality or fitment", with screws loosened to free L/R; another reports "bends and gaps in places it shouldnt". Source: eBay listings via search ([example](https://ebay.com/itm/Nintendo-DS-Lite-Full-Replacement-Housing-Shell-Screen-Lens-Clear-Yellow-US/202452427135)) (snippet). COMMUNITY, weak.
- **Matched faceplate needed.** One reviewer warns a faceplate "doesn't fit on all shells", so the bottom faceplate must match the shell. Source: eBay listing via search (snippet). COMMUNITY, weak.
- **No measured comparison.** No source measured wall thickness, gloss or colour difference between genuine and aftermarket shells (searches above). Gap noted.

### Inferences
- **Why replicas feel different.** A single-shot ABS replica cannot reproduce the genuine "glass cover" depth, because there is no clear skin over the colour. This is likely the main visual difference beyond thickness and fit.
- **Lessons for a printed case.** Robust hinge knuckles, adequate boss walls, and controlled screw torque or heat-set inserts are where replicas most often fail.

### Gaps
- No quantitative comparison (callipers, scale or gloss meter) of genuine vs aftermarket parts was found.
- The GameFAQs thread "Original Shell vs Aftermarket Shell" could not be read (HTTP 403): [GameFAQs](https://gamefaqs.gamespot.com/boards/925329-nintendo-ds/76520478).

## 8. Reproducing the look and feel on 3D-printed parts: finishing processes, durability and comfort, material stiffness and density, quantitative data

### Takeaway
No gloss-unit or roughness measurement of a DS Lite (or a similar console) was found.

**Gloss target.** The ISO 2813 convention puts "high gloss" above about 70 GU at 60 degrees. The DS Lite's "glass cover" gloss almost certainly sits there (ESTIMATED).

**Optics.** The genuine optic is a clear skin over colour, so the closest printed analogue is a coloured, sanded-smooth base under a thick, polished clear coat. Two hobby-accessible coats fit that role:
- a 2K (isocyanate-cured) acrylic clear, which is hard, chemical resistant and polishable;
- a brush-on epoxy (XTC-3D, Shore D 80).

These work on PLA and PETG, which are the owner's current materials.

**Alternatives:**
- Acetone vapour smoothing gives an "injection-moulded look" only on ABS or ASA. It rounds detail and adds no depth.
- Ironing only smooths flat top surfaces.
- SLA resin can be sanded to a reflective finish.

**Material match.** ABS, ASA or PC-ABS filament comes closest to the genuine material's density (about 1.04 to 1.14 g/cm3) and stiffness (about 2.0 to 2.3 GPa flexural). PLA is about 15 to 20 percent denser and noticeably stiffer; PETG is denser and less stiff.

### Cited Findings
Gloss measurement reference:
- **ISO 2813 geometry.** The 20 degree geometry is intended for high-gloss films "with a 60° specular gloss higher than about 70 units"; 85 degrees for films below about 10 units at 60 degrees. Sources: [ISO 2813:1994 abstract (ANSI webstore)](https://webstore.ansi.org/standards/iso/iso28131994) (snippet) and [Polymer Service Merseburg wiki, Gloss Measurement](https://en.wiki.polymerservice-merseburg.de/index.php/Gloss_Measurement) (snippet: the 60 degree geometry is permitted for all coatings; values are in GU). OFFICIAL (standard), via secondary summaries.
- **Commercial gloss bands.** One blog gives High Gloss above 70 GU, Semi-Gloss 40 to 70, Satin 10 to 40 and Matte below 10 GU, all at 60 degrees. Source: [cypaint blog](https://cypaint.com/article/how-to-check-gloss-level-of-paint) (snippet). COMMUNITY. It disagrees with the 10 to 70 GU "semi-gloss" band used elsewhere.

Ironing (FDM):
- **What it does.** Prusa: ironing adds a hot-nozzle pass that smooths flat top surfaces only. Edges get "slightly fuzzy". It is not useful on round, organic or non-horizontal surfaces, and adds print time. PETG builds up on the nozzle; ASA gives "super smooth top surfaces". Source: [Prusa Knowledge Base, Ironing](https://help.prusa3d.com/article/ironing_177488). COMMUNITY (vendor documentation).

Acetone vapour smoothing (ABS or ASA only):
- **Effect on the surface.** It dissolves the outermost 0.1 to 0.3 mm. Works only on ABS and ASA, not PLA, PETG, TPU or nylon. Cold vapour reaches full gloss in 4 to 8 h; heated vapour in 10 to 30 min, with a much higher fire risk. It can give a "mirror-smooth, injection-molded look". It adds 0.05 to 0.15 mm to external dimensions and rounds detail; walls under 1.5 mm are at risk. Parts stay soft for hours, so allow 12 to 24 h before handling. Source: [3dprinterstuff, vapor smoothing ABS](https://www.3dprinterstuff.com/workshop/vapor-smoothing-abs-prints). COMMUNITY (blog).
- **Roughness data.** A conference paper reports a 3D-printed part's roughness falling from 14.63 µm to 0.12 µm after acetone treatment. The setup used liquid acetone with an ultrasonic transducer, and the polymer was not clear from the extract. Source: [Kesvarakul 2019, Key Engineering Materials 821](https://is.rajapark.ac.th/assets/uploads/66696f48676b7kesvarakul2019_(1).pdf) (snippet). ESTIMATED applicability; not verified.

Epoxy coating (any rigid print, including PLA and PETG):
- **XTC-3D data.** Mix 2A:1B by volume; pot life 10 min; cure about 3.5 h (about 15 min at 60 °C); Shore D 80; specific gravity 1.09; under 1/64 in (0.4 mm) per coat. A "smooth, high gloss finish" from one coat. Can be sanded, primed and painted. Works on PLA, ABS and SLA prints. Source: [Smooth-On XTC-3D product page](https://www.smooth-on.com/products/xtc-3d/). COMMUNITY (manufacturer data).
- **XTC-3D vs vapour (hobbyist).** On ABS, XTC-3D gave a better coating with fewer areas to fix than a vapour bath, and priming then revealed spots that still needed sanding. Source: [Chomick+Meder blog](https://chomickmeder.com/blg-3d-prints-smooth/) (snippet). COMMUNITY.

2K clear coat (over primer, colour base and sanding):
- **SprayMax 2K Clear Glamour (manufacturer data sheet):**
  - a "high-gloss 2-component clear coat" with "long-lasting resistance to weathering and to chemicals; it is very easy to polish";
  - resin is a two-component acrylic with a hardener "containing isocyanates";
  - pot life 48 h at 20 °C after activation; 1 to 2 spray passes of about 0.6 mil each, 5 to 7 min flash between coats;
  - touch dry 18 h at 20 °C, or forced dry 60 min at 60 °C;
  - an A2/P2 respirator and gloves are required, with a warning about isocyanate sensitisation; "For professional use only".
  - Source: [SprayMax 2K Clear Glamour spec sheet, Prod. no. 368 0061, v01/2018](https://nodonline.nationaloak.com/media/import/MFRSPRM/Specification_Sheet/spraymax_3680061_specification_sheet.pdf). COMMUNITY (manufacturer data). FLAG: the sheet's coverage line says "dry film thickness of 1.2 to 2.0 µm", which is inconsistent with 0.6 mil per pass. It is probably a unit typo for mil.
- **Splash Paints 2K Ultra Clear.** Two-part urethane for models and 3D prints; 3:1 mix; gloss and matte; vendor claims 10+ years of outdoor durability with no yellowing. Source: [La Scale Model listing](https://lascalemodel.com/products/splash-paints-2k-2-part-ultra-clear-coat-clear-coat-30ml-catalyst-10ml) (snippet). COMMUNITY (vendor).
- **Isocyanate-free alternative.** Würth's aerosol clear claims high gloss and UV resistance with no isocyanates. Source: [Würth Industry listing](https://shop.wurthindustry.com/products/wurth-clear-coat-12-clear-coat-12fl-oz-24-unit-pack) (snippet). COMMUNITY (vendor).
- **Test bonding on PLA first.** Prop makers recommend 2-part urethane clears (SprayMax named) for durable PLA finishes, with a test that the urethane bonds to the top coat on a sample print and a light sand first. Source: [Stan Winston School forum](https://forums.stanwinstonschool.com/discussion/comment/8326/) (snippet). COMMUNITY.
- **ColorFabb.** Sand prints lightly with fine grit to smooth layer lines before coating; use a UV-resistant clear for sunlit parts. Source: [ColorFabb blog, how to coat a print](https://colorfabb.com/blog/post/how-to-coat-a-print) (snippet). COMMUNITY.

IPA-smoothable PVB filament:
- **Polymaker PolySmooth with the Polysher.** The Polysher nebulises IPA to polish PolySmooth prints to "a smooth, glossy finish". Time is 20 to 40 min per [MatterHackers](https://matterhackers.com/store/l/polysmooth-filament-white-175mm/sk/MZTCXYQK) (snippet), or 10 to 30 min per [filament2print](https://filament2print.com/gb/accesorios/835-polysher.html) (snippet). A [Fabbaloo hands-on, 2016](https://www.fabbaloo.com/blog/2016/12/14/inspecting-polymakers-smoothing-system) warns against breathing the dense mist (snippet). COMMUNITY. No gloss or roughness numbers found.

SLA resin route:
- **Formlabs.** SLA parts "can be sanded to be smooth and glossy with 3,000 grit sandpaper"; "At around 12,000 grit, the parts should become reflective". Clear spray coats are "the best option for a glistening finish": two to three coats after an IPA wash and full dry. For Clear Resin, skip post-cure before coating to avoid yellowing. Source: [Formlabs, post-processing and finishing SLA prints](https://formlabs.com/blog/post-processing-and-finishing-sla-prints/). COMMUNITY (vendor guidance).

Material density and stiffness (printed vs genuine-type plastics):
- **Polymaker densities (g/cm3) and HDT at 0.45 MPa:**
  - PolyLite PLA 1.17 (60 °C); PolyLite PLA Pro 1.23 (59 °C);
  - PolyLite PETG 1.25 (78 °C);
  - PolyLite ABS 1.04 (100 °C);
  - Polymaker ASA 1.13 (103 °C);
  - PolyLite PC 1.19 (111 °C); Polymaker PC-ABS 1.1 (112 °C).
  - Source: [Polymaker wiki, technical data at a glance](https://wiki.polymaker.com/polymaker-wiki/polymaker-products/polymaker-filaments/technical-data-at-a-glance). COMMUNITY (manufacturer data).
- **Generic FDM values, density (g/cm3) and flexural modulus (MPa):** PLA 1.24 / 3200; PETG 1.24 / 1600; ABS 1.05 / 2000; ASA 1.07 / 2000; PC 1.20 / 2200; PC-ABS 1.14 / 2350. No brand given. Source: [4dfiltration, FDM properties](https://4dfiltration.com/resources/fdm-properties). ESTIMATED (generic).
- **Ultimaker Method flexural modulus:** PETG 2050 MPa, ABS-R 2600 MPa, ASA 2300 MPa. Source: [Ultimaker materials](https://ultimaker.com/materials/method-series-pc-abs-fr) (snippet). COMMUNITY (manufacturer).
- **Injection-moulding PC/ABS grades (reference for the genuine-type material):**
  - Elix PC/ABS 5130: about 1.13 g/cm3, 2280 MPa flexural ([Knowde](https://knowde.com/stores/elix-polymers/products/elix-pc-abs-5130), snippet).
  - Romiloy 1045 F PG: 1.09 g/cm3, 1800 MPa ([Knowde](https://knowde.com/stores/romira/products/romiloy-1045-f-pg), snippet).
  - Dinablend PC/ABS FR: 1.20 g/cm3, 2200 MPa ([datasheet](https://www.polydisteurope.com/assets/uploads/files/PCABS_1S15V0-7035_Dinablend.pdf), snippet).
  - Kumho Ecoblend HAC8251NH is marketed for monitor, printer and mobile-phone housings ([SpecialChem](https://www.specialchem.com/plastics/product/shanghai-kumho-sunny-plastics-ecoblend-hac8251nh), snippet).
  - COMMUNITY (manufacturer data).

Comfort, from the original's users:
- **Gloss and grip.** Shacknews felt the glossy shell made the reviewer "very conscious about how you hold the thing" ([Shacknews](https://www.shacknews.com/article/41445/nintendo-ds-lite-enamel-navy)). An owner preferred the textured inner casing's feel over "slippery smooth plastic" ([bit-tech](https://bit-tech.net/news/modding/Scratched_up_DS_Lite_Mod/1)). NWR found the unit small for adult hands, though light ([NWR](https://www.nintendoworldreport.com/review/11365)). OBSERVED and COMMUNITY.

### Inferences
Each route compared on how close it gets, given the owner's PLA and PETG today:
- **Closest optical match, PLA or PETG: colour base under a thick 2K clear.**
  - Print the back and sides in the target colour, or prime and paint the colour.
  - Fill and sand layer lines (filler primer, or one XTC-3D coat sanded flat), then apply a 2K clear and wet-sand and polish it.
  - The clear thickness recreates the genuine part's "glass cover" depth from the clear shot over colour.
  - Durability: a cured 2K acrylic-urethane is the hardest and most chemical-resistant hobby option listed. It also tolerates hand oils better than 1K lacquers (ESTIMATED; no head-to-head test found).
  - Cost: isocyanate safety (A2/P2 respirator, ventilation).
- **XTC-3D alone.** It gives a high gloss in one or two coats with no isocyanates (Shore D 80). It is thicker and less even than a sprayed clear, so a final sanded and polished clear over it gets nearer an injection-moulded flatness.
- **Vapour smoothing.** Viable only if the owner switches to ABS or ASA. It gives gloss without adding a coating, but rounds edges and button-hole detail (masking needed). The colour is the plastic itself, so there is no depth effect. Walls under 1.5 mm risk deforming, which matters for a 1.2 to 1.5 mm DS Lite-like wall.
- **Ironing.** Only helps flat top surfaces. Of the bottom half, that could be the matte face if printed face-up, not the curved back. It does not create a gloss comparable to injection moulding.
- **Silk and "glossy" filaments.** Not evaluated here (see gaps). Layer lines remain on walls, so they cannot give the flat mirror finish alone.
- **SLA resin.** Resin (for example via a print service) sanded to 3000 grit and clear coated gets a smooth base fastest. Long-term resin brittleness and yellowing vary by resin.
- **Matte face.** Leave a satin or matte finish around the controls (a matte clear, or a fine sanded finish), copying Nintendo's deliberate gloss outside and matte inside split. This also answers the grip complaint about the glossy shell.
- **Weight and stiffness feel.** At the same wall thickness:
  - PLA is about 15 to 20 percent denser (1.17 to 1.24 vs about 1.04 to 1.14 g/cm3) and stiffer (about 3.2 vs 2.0 to 2.3 GPa) than ABS or PC-ABS;
  - PETG is denser but less stiff (about 1.6 to 2.05 GPa);
  - ABS, ASA or PC-ABS filament comes closest to the genuine-type plastic.
  - Printed walls are anisotropic and may have voids, so effective stiffness is lower than these bulk numbers (ESTIMATED). A coating such as 2K clear or epoxy adds a little mass and surface hardness.
- **Heat.** PLA's 60 °C HDT is low for a handheld left in a hot car. ABS, ASA, PC-ABS and PC (100 to 114 °C) match injection-moulded handheld plastics better.

### Gaps
- No gloss-unit (GU) or Ra measurement of a genuine DS Lite, or of a comparable glossy handheld (PSP, iPod), was found.
- No gloss or Ra data was found for:
  - FDM PLA or PETG as printed, or ironed;
  - silk or "high gloss" PLA or PETG filaments;
  - PolySmooth after IPA;
  - 2K-clear-coated prints.
- No controlled comparison of 2K clear vs XTC-3D vs 1K lacquer on PLA, for scratch resistance, sweat and oil resistance, or long-term adhesion.
- No published hobbyist project reproducing the DS Lite's clear-over-colour two-layer look on a 3D print was found.
- Scratch hardness (pencil hardness) data for these coatings was not found, except Shore D 80 for XTC-3D.
