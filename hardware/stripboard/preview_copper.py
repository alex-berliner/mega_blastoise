"""Copper-side-up view: the board flipped over (mirrored left-right), i.e.
what you actually look at while cutting tracks and soldering. Printed letters
read A..X left to right in this view. Cuts, jumpers and solder points are
solid; components are ghosts (they sit on the far side)."""
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, FancyBboxPatch
import gen_single_board as S

def rgbf(t): return tuple(v/255 for v in t)
NC, NH = S.NCOLS, S.NHOLES
def X(c): return (NC-1-c)*0.1          # mirror horizontally
def Y(h): return h*0.1

fig,ax = plt.subplots(figsize=(11,22))
ax.set_xlim(-0.55,2.55); ax.set_ylim(0.0,5.75); ax.set_aspect('equal'); ax.axis('off')
ax.invert_yaxis()

# board + copper strips (solid: this side IS the copper)
ax.add_patch(FancyBboxPatch((X(NC-1)-0.05,Y(1)-0.05),2.3+0.10,Y(NH-1)+0.10,
             boxstyle="round,pad=0.01,rounding_size=0.05",fc="#f0e0c8",ec="#cbbf9a",lw=1.5,zorder=0))
for c in range(NC):
    ax.add_patch(Rectangle((X(c)-0.028,Y(1)),0.056,Y(NH-1),fc="#da8a67",ec="#b06a4a",lw=0.3,alpha=0.85,zorder=0.5))
    ax.text(X(c),Y(0.3),S.letter(c),ha="center",va="center",fontsize=6.5,color="#555",fontweight="bold")
for h in range(1,NH+1):
    for c in range(NC):
        ax.add_patch(Circle((X(c),Y(h)),0.012,fc="#f0e0c8",ec="none",zorder=0.9))
for h in range(5,NH+1,5):
    ax.text(X(NC-1)-0.14,Y(h),str(h),ha="center",va="center",fontsize=5.5,color="#999")
    ax.text(X(0)+0.14,Y(h),str(h),ha="center",va="center",fontsize=5.5,color="#999")

# ghost components (they are on the FAR side)
for (nm,c0,h0,c1,h1) in S.MODULES:
    x0=min(X(c0),X(c1)); w_=abs(X(c1)-X(c0))
    ax.add_patch(FancyBboxPatch((x0,Y(h0)),w_,Y(h1-h0),boxstyle="round,pad=0.005,rounding_size=0.02",
                 fc="none",ec="#667",lw=1.0,ls=(0,(3,3)),alpha=0.6,zorder=2))
for (c0,h0,c1,h1) in S.SW_BODIES:
    x0=min(X(c0),X(c1)); w_=abs(X(c1)-X(c0))
    ax.add_patch(FancyBboxPatch((x0-0.03,Y(h0)),w_+0.06,Y(h1-h0),boxstyle="round,pad=0.004,rounding_size=0.02",
                 fc="none",ec="#888",lw=0.8,ls=(0,(2,2)),alpha=0.6,zorder=2))
px0=min(X(S.PC0-0.5),X(S.PC0+19.5)); pw=abs(X(S.PC0+19.5)-X(S.PC0-0.5))
ax.add_patch(FancyBboxPatch((px0,Y(S.PTOP-0.7)),pw,Y(S.PBOT-S.PTOP+1.4),
             boxstyle="round,pad=0.01,rounding_size=0.03",fc="none",ec="#556",lw=1.2,ls=(0,(4,3)),alpha=0.7,zorder=2))
ax.text(px0+pw/2,Y((S.PTOP+S.PBOT)/2),"Pico (on the FAR side - USB on the RIGHT in this view)",
        ha="center",va="center",fontsize=7.5,color="#556",zorder=3)

# jumper wires (THIS side)
for j in S.JUMPERS:
    (c1,h1),(c2,h2)=j['a'],j['b']
    ax.plot([X(c1),X(c2)],[Y(h1),Y(h2)],color=rgbf(S.netrgb(j['net'])),lw=1.5,zorder=5,alpha=0.95)
# solder points (pads = where legs/wires get soldered on THIS side)
for p in S.PADS:
    gray = p['net'].startswith('x')
    ax.add_patch(Circle((X(p['c']),Y(p['h'])),0.020 if gray else 0.027,
                 fc="#999" if gray else rgbf(S.netrgb(p['net'])),ec="#222",lw=0.4,zorder=6))
# track cuts (THIS side)
for (c,h) in S.CUTS:
    ax.plot([X(c)-0.032,X(c)+0.032],[Y(h)-0.032,Y(h)+0.032],color="red",lw=2.0,zorder=7)
    ax.plot([X(c)-0.032,X(c)+0.032],[Y(h)+0.032,Y(h)-0.032],color="red",lw=2.0,zorder=7)

ax.text(X(NC-1),Y(0.3)-0.09,"COPPER SIDE UP - the side you cut and solder.  Letters read A..X; layout is the mirror of the component-side diagrams.",
        ha="left",va="bottom",fontsize=8.6,fontweight="bold")
ax.text(X(NC-1),5.68,"Solid = on this side (strips, cuts X, wires, solder points).  Dashed = parts on the far side.",
        ha="left",va="center",fontsize=7.5,color="#445")
plt.tight_layout()
out="/home/alex/Code/mega_blastoise/hardware/stripboard/mega_blastoise_single_board_copper.png"
plt.savefig(out,dpi=130,bbox_inches="tight",facecolor="white"); print("saved",out)
