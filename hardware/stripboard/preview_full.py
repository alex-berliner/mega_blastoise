import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, FancyBboxPatch
import gen_matrix_full_diy as F
g = F.g
T, ROWS, COLS = g.T, g.ROWS, g.COLS
node_of, xL, xgap, HSPAN = g.node_of, g.xL, g.xgap, g.HSPAN
def rgbf(t): return tuple(v/255 for v in t)

BX0,BY0,BX1,BY1 = 0.0,0.30,2.3,2.60
fig,ax = plt.subplots(figsize=(15,20))
ax.set_xlim(-0.4,5.6); ax.set_ylim(-0.05,5.95); ax.set_aspect('equal'); ax.axis('off')
ax.invert_yaxis()

# board + vertical strips
ax.add_patch(FancyBboxPatch((BX0,BY0),BX1-BX0,BY1-BY0,boxstyle="round,pad=0.01,rounding_size=0.05",
             fc="#f4f0e4",ec="#cbbf9a",lw=2,zorder=0))
letters="ABCDEFGHIJKLMNOPQRSTUVWX"; x=BX0; li=0
while x<=BX1+1e-6:
    ax.add_patch(Rectangle((x-0.012,BY0),0.024,BY1-BY0,fc="#da8a67",ec="none",alpha=0.35,zorder=0.5))
    if li<len(letters): ax.text(x,BY0-0.06,letters[li],ha="center",va="center",fontsize=6,color="#888")
    x=round(x+0.1,2); li+=1

# buttons
for (p,k,nc,gp,by) in ROWS:
    ny=node_of(by)
    for c in range(nc):
        rx,ryv=T(xL(c),by); cx,cyv=T(round(xL(c)+HSPAN,2),ny)
        x0=min(rx,cx); y0=min(ryv,cyv)
        ax.add_patch(FancyBboxPatch((x0-0.06,y0-0.06),abs(rx-cx)+0.12,abs(ryv-cyv)+0.12,
                     boxstyle="round,pad=0.005,rounding_size=0.03",fc="#3a3a3a",ec="#111",lw=1,alpha=0.9,zorder=3))
        for lx in (rx,cx):
            for ly in (ryv,cyv): ax.add_patch(Circle((lx,ly),0.024,fc="#999",ec="#333",lw=0.5,zorder=4))
        ax.add_patch(Circle((rx,ryv),0.028,fc="#00b000",ec="#024",lw=0.6,zorder=5))
        ax.add_patch(Circle((cx,cyv),0.028,fc="#0060d0",ec="#012",lw=0.6,zorder=5))
        lx,ly=T(round(xL(c)+HSPAN/2,2),round((by+ny)/2,2))
        ax.text(lx,ly,f"{k}{c+1}",color="white",ha="center",va="center",fontsize=6,fontweight="bold",zorder=6)

# cuts + col jumpers + header pads
for (p,k,nc,gp,by) in ROWS:
    ny=node_of(by)
    for c in range(nc-1):
        cx,cyv=T(round(xL(c)+HSPAN+0.1,2),ny)
        ax.plot([cx-0.03,cx+0.03],[cyv-0.03,cyv+0.03],color="red",lw=2,zorder=7)
        ax.plot([cx-0.03,cx+0.03],[cyv+0.03,cyv-0.03],color="red",lw=2,zorder=7)
for (c,gp) in COLS:
    nodes=sorted(node_of(by) for (p,k,nc,gg,by) in ROWS if c<nc)
    pts=[T(xgap(c),n) for n in nodes]
    ax.plot([q[0] for q in pts],[q[1] for q in pts],color="#0050c0",lw=2,zorder=5)
    for q in pts: ax.add_patch(Circle(q,0.024,fc="#0050c0",ec="none",zorder=6))
for gp,(hx,hy) in F.HEADER.items():
    ax.add_patch(Circle((hx,hy),0.036,fc="#e06000",ec="#500",zorder=6))

# all wires (matrix + modules), as bezier-ish polylines through control points
from matplotlib.path import Path
import matplotlib.patches as mpatches
for (name,rgb,pts) in F.WIRES:
    verts=pts; codes=[Path.MOVETO,Path.CURVE4,Path.CURVE4,Path.CURVE4]
    ax.add_patch(mpatches.PathPatch(Path(verts,codes),fill=False,edgecolor=rgbf(rgb),lw=1.8,zorder=8))

# Pico body + used pins
px0,py0,row=F.PICO_X0,F.PICO_Y0,F.PICO_ROW
ax.add_patch(FancyBboxPatch((px0-0.11,py0-0.11),row+0.22,1.9+0.22,boxstyle="round,pad=0.01,rounding_size=0.04",
             fc="#464650",ec="#28282d",lw=1.5,zorder=7))
ax.text(px0+row/2,py0-0.02,"Raspberry Pi Pico",color="#e6e6e6",ha="center",va="center",fontsize=7.5,fontweight="bold",zorder=9,rotation=90)
for pin in range(1,41):
    x,y=F.pin_xy(pin)
    if pin in F.USED_PINS:
        lab,rgb=F.USED_PINS[pin]
        ax.add_patch(Circle((x,y),0.030,fc=rgbf(rgb),ec="#111",lw=0.6,zorder=9))
        if pin<=20: ax.text(x-0.05,y,f"{lab} p{pin}",ha="right",va="center",fontsize=5.6,color=rgbf(rgb),fontweight="bold",zorder=9)
        else:       ax.text(x+0.05,y,f"{lab} p{pin}",ha="left",va="center",fontsize=5.6,color=rgbf(rgb),fontweight="bold",zorder=9)
    else:
        ax.add_patch(Circle((x,y),0.020,fc="#999",ec="#333",lw=0.4,zorder=8))

# modules
for (tag,x0,y0,x1,y1,title) in F.MODULE_RECTS:
    ax.add_patch(FancyBboxPatch((x0,y0),x1-x0,y1-y0,boxstyle="round,pad=0.005,rounding_size=0.03",
                 fc="#232838",ec="#11141f",lw=1.2,alpha=0.95,zorder=6))
    ax.text(x0+0.04,y0-0.09,title,ha="left",va="center",fontsize=7,fontweight="bold",color="#333",zorder=9)
for (x,y,rgb) in F.MODULE_PADS:
    ax.add_patch(Circle((x,y),0.030,fc=rgbf(rgb),ec="#111",lw=0.5,zorder=8))
for (x,y,text,rgb) in F.MODULE_PAD_LABELS:
    ax.text(x+0.03,y,text,ha="left",va="center",fontsize=5.6,color="white",fontweight="bold",zorder=9)

# parts list
for i,line in enumerate(F.PARTS):
    ax.text(3.6,round(3.45+0.22*i,2),line,ha="left",va="center",
            fontsize=9 if i==0 else 7.5,fontweight="bold" if i==0 else "normal",color="#222",zorder=9)

ax.text(0.0,-0.04,"Mega Blastoise - full system: matrix + Pico + 2 OLEDs + 2 WS2812B strips + buzzer",
        ha="left",va="bottom",fontsize=10,fontweight="bold")
plt.tight_layout()
out="/home/alex/Code/mega_blastoise/hardware/stripboard/mega_blastoise_matrix_full_preview.png"
plt.savefig(out,dpi=130,bbox_inches="tight",facecolor="white"); print("saved",out)
