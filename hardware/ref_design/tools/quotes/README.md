# Board quotes

Headless runs of JLCPCB's and PCBWay's instant-quote forms, so the board price can be
re-checked when the design changes. Only the form is filled in: nothing is uploaded, and
the scripts never sign in or order.

    npm install                  # playwright-core 1.52 (the last to run on Node 18)
    node jlc.js 4 10 ENIG "0.25mm/(0.35/0.4mm)" jlc_4l.png   # layers, qty, finish, min via option
    node jlc.js 2 10 ENIG default jlc_2l.png
    node pcbway.js "4 Layers" 10 "0.3mm" pw_4l.png          # layers, qty, min hole
    node pcbway_assembly.js 10 40 91 6 1                    # qty, unique parts, placements, QFN/BGA, through-hole

The board size (70.5 x 151.34 mm), 1.6 mm, ENIG and green are set in the scripts; change
them there if the outline changes. Each run prints the itemised price and saves a
screenshot, which is the thing to check: both sites change their forms, and a click that
lands on the wrong option (JLC's quantity list shares numbers with its layer buttons)
quotes the wrong board without any error.

`CHROME` overrides the browser path (default: Playwright's cached Chromium).

Results go on the cost-decisions page (collection `quotes`) and into `docs/bom.md`.
