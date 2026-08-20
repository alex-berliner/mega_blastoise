#!/usr/bin/env bash
# Build the solid models from the board geometry, and look at them.
#
#   ./build.sh          regenerate, render the contact sheet, export STLs
#   ./build.sh render   contact sheet only, seconds, no STL
#   ./build.sh stl      STLs only
#
# Everything downstream of layout.py is disposable; the geometry lives there.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
what="${1:-all}"

step() { printf '\n== %s\n' "$1"; }

step "regenerate the .scad from layout.py"
( cd "$root" && python3 layout.py >/dev/null )
ls -la "$here"/*.scad

if [ "$what" = all ] || [ "$what" = render ]; then
    step "contact sheet"
    # preview mode, so this stays fast; it is for looking at, not for slicing
    render() {
        openscad -o "$here/renders/$1.png" --autocenter --viewall \
                 --camera=0,0,0,"$2" --imgsize="$3" \
                 ${4:+-D "$4"} "$here/$5" 2>&1 |
            grep -E 'ECHO|ERROR|WARNING|Assertion' || true
    }
    render assembled_iso    58,0,205,0  1100,1150 'show_copper=true'  assembled.scad
    render assembled_top    0,0,0,0     760,1620  'show_copper=true'  assembled.scad
    render assembled_board  58,0,205,0  1100,1150 'part="board"'      assembled.scad
    render assembled_deck   58,0,25,0   1100,1150 'part="parts"'      assembled.scad
    render stencil_top      0,0,0,0     760,1620  ''                  cuts_stencil.scad
    render stencil_iso      58,0,205,0  1100,1150 ''                  cuts_stencil.scad
    echo "look at every png in renders/ before trusting any of it"
fi

if [ "$what" = all ] || [ "$what" = stl ]; then
    step "stencil STL (seconds)"
    time openscad -o "$here/stl/cuts_stencil.stl" "$here/cuts_stencil.scad"

    step "assembled STL (about 75 s: 1344 drilled holes through CGAL)"
    time openscad -o "$here/stl/assembled.stl" "$here/assembled.scad"

    step "assembled STL, undrilled (about 12 s, for a quick fit check)"
    time openscad -o "$here/stl/assembled_plain.stl" \
         -D 'show_holes=false' "$here/assembled.scad"

    ls -la "$here"/stl/
fi
