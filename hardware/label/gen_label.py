#!/usr/bin/env python3
"""Build back_label.svg from the template by embedding the fonts as base64,
then render the print PNG (600 dpi) and PDF (65x145 mm) via headless chromium."""
import base64
import pathlib
import shutil
import subprocess

HERE = pathlib.Path(__file__).parent

def b64(name: str) -> str:
    return base64.b64encode((HERE / name).read_bytes()).decode()

svg = (HERE / "back_label_template.svg").read_text()
svg = svg.replace("@@LUCKIEST_B64@@", b64("luckiestguy.ttf"))
svg = svg.replace("@@BALOO_B64@@", b64("baloo2.ttf"))
(HERE / "back_label.svg").write_text(svg)

# 65 x 145 mm at 600 dpi = 1535 x 3425 px
(HERE / "_shot.html").write_text(
    '<!doctype html><html><head><style>*{margin:0;padding:0}</style></head>'
    '<body><img src="back_label.svg" style="width:1535px;height:3425px;display:block">'
    "</body></html>"
)
(HERE / "_print.html").write_text(
    "<!doctype html><html><head><style>@page{size:65mm 145mm;margin:0}"
    "*{margin:0;padding:0}</style></head>"
    '<body><img src="back_label.svg" style="width:65mm;height:145mm;display:block">'
    "</body></html>"
)
print("wrote back_label.svg (%d KB)" % (len(svg) // 1024))

chromium = shutil.which("chromium-browser") or shutil.which("chromium")
if not chromium:
    raise SystemExit("chromium not found; back_label.png/.pdf not regenerated")

common = [chromium, "--headless", "--disable-gpu", "--hide-scrollbars"]
subprocess.run(
    common + ["--window-size=1535,3425", "--screenshot=back_label.png",
              f"file://{HERE}/_shot.html"],
    cwd=HERE, check=True, capture_output=True,
)
subprocess.run(
    common + ["--no-pdf-header-footer", "--print-to-pdf=back_label.pdf",
              f"file://{HERE}/_print.html"],
    cwd=HERE, check=True, capture_output=True,
)
print("wrote back_label.png (600 dpi) and back_label.pdf (65x145 mm)")
