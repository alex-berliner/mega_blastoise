"""Minimal RS-274X gerber rasteriser, for reading other people's boards.

Written to measure membrane contact pads out of gerbers when no source files
exist. Parses apertures, G36/G37 regions, draws and flashes; approximates arcs
as line segments, which is fine for measurement and for looking at a layer.

    python3 gerb.py out.png            # renders the DS Lite board in $PWD
    from gerb import parse, render     # or drive it from your own script

parse() returns (apertures, regions, draws, flashes) in mm.
"""
import re, sys
from PIL import Image, ImageDraw

SCALE=10**4  # FSLAX34
PPMM=40

def parse(fn):
    txt=open(fn).read()
    aps={}
    for m in re.finditer(r'%ADD(\d+)([A-Z]+),?([^*]*)\*%', txt):
        aps[int(m.group(1))]=(m.group(2), m.group(3))
    x=y=0.0; ap=None; inreg=False; cur=[]
    regions=[]; draws=[]; flashes=[]
    for line in txt.splitlines():
        line=line.strip()
        if line.startswith('G36'): inreg=True; cur=[]; continue
        if line.startswith('G37'):
            if len(cur)>2: regions.append(cur)
            inreg=False; cur=[]; continue
        md=re.match(r'^(?:G\d+)?D(\d+)\*$', line)
        if md and int(md.group(1))>=10: ap=int(md.group(1)); continue
        m=re.match(r'^(?:G\d+)?(?:X(-?\d+))?(?:Y(-?\d+))?(?:I(-?\d+))?(?:J(-?\d+))?D0?(\d)\*$', line)
        if not m: continue
        px,py=x,y
        if m.group(1) is not None: x=int(m.group(1))/SCALE
        if m.group(2) is not None: y=int(m.group(2))/SCALE
        op=m.group(5)
        if inreg:
            if op=='2':
                if len(cur)>2: regions.append(cur)
                cur=[(x,y)]
            else: cur.append((x,y))
        else:
            if op=='1': draws.append(((px,py),(x,y),ap))
            elif op=='3': flashes.append(((x,y),ap))
    return aps, regions, draws, flashes

def apsize(aps, ap):
    if ap not in aps: return 0.2
    t,p=aps[ap]
    nums=[float(v) for v in p.split('X') if v.strip()]
    return nums[0] if nums else 0.2

def render(files, out, w_mm, h_mm, x0=0.0, y0=0.0):
    W=int(w_mm*PPMM); H=int(h_mm*PPMM)
    img=Image.new("RGB",(W,H),(20,20,24)); d=ImageDraw.Draw(img)
    def T(p): return ((p[0]-x0)*PPMM, H-(p[1]-y0)*PPMM)
    for fn, col in files:
        aps, regions, draws, flashes = parse(fn)
        for r in regions:
            d.polygon([T(p) for p in r], fill=col)
        for a,b,ap in draws:
            wpx=max(1,int(apsize(aps,ap)*PPMM))
            d.line([T(a),T(b)], fill=col, width=wpx)
        for (p,ap) in flashes:
            t,par=aps.get(ap,('C','0.5'))
            nums=[float(v) for v in par.split('X') if v.strip()]
            cx,cy=T(p)
            if t=='R' and len(nums)>=2:
                hw,hh=nums[0]*PPMM/2, nums[1]*PPMM/2
                d.rectangle([cx-hw,cy-hh,cx+hw,cy+hh], fill=col)
            else:
                r=(nums[0] if nums else 0.5)*PPMM/2
                d.ellipse([cx-r,cy-r,cx+r,cy+r], fill=col)
    img.save(out); print("wrote", out, img.size)

if __name__=="__main__":
    base="DS_Lite_Membrane_PCB-Ampersand"
    render([(base+".GTL",(200,140,60)),(base+".GKO",(90,200,120))], sys.argv[1], 32, 35)
