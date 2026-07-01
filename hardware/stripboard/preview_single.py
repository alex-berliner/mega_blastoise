import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, FancyBboxPatch
import gen_single_board as S

def rgbf(t): return tuple(v/255 for v in t)
NC, NH = 24, 55
fig,ax = plt.subplots(figsize=(11,22))
ax.set_xlim(-0.6,2.5); ax.set_ylim(0.0,5.7); ax.set_aspect('equal'); ax.axis('off')
ax.invert_yaxis()

# board + vertical strips
ax.add_patch(FancyBboxPatch((0.0,0.1),2.3,5.4,boxstyle="round,pad=0.01,rounding_size=0.05",
             fc="#f4f0e4",ec="#cbbf9a",lw=1.5,zorder=0))
letters="ABCDEFGHIJKLMNOPQRSTUVWX"
for c in range(NC):
    x=c*0.1
    ax.add_patch(Rectangle((x-0.012,0.1),0.024,5.4,fc="#da8a67",ec="none",alpha=0.30,zorder=0.5))
    ax.text(x,0.03,letters[c],ha="center",va="center",fontsize=6,color="#888")
for h in range(1,NH+1,5):
    ax.text(-0.10,h*0.1,str(h),ha="center",va="center",fontsize=5.5,color="#aaa")

def X(c): return c*0.1
def Y(h): return h*0.1

# Pico body
ax.add_patch(FancyBboxPatch((X(S.PC0)-0.06,Y(S.PTOP)-0.05),X(19)+0.12,Y(S.PBOT-S.PTOP)+0.10,
             boxstyle="round,pad=0.01,rounding_size=0.03",fc="#464650",ec="#28282d",lw=1.3,zorder=2))
ax.text(X(S.PC0+9),Y((S.PTOP+S.PBOT)/2),"Raspberry Pi Pico (underside)",ha="center",va="center",
        fontsize=8,fontweight="bold",color="#eee",zorder=6)
# module + switch bodies
for (nm,c0,h0,c1,h1) in S.MODULES:
    ax.add_patch(FancyBboxPatch((X(c0),Y(h0)),X(c1-c0),Y(h1-h0),boxstyle="round,pad=0.005,rounding_size=0.02",
                 fc="#232838",ec="#11141f",lw=1.0,zorder=2))
for (c0,h0,c1,h1) in S.SW_BODIES:
    ax.add_patch(FancyBboxPatch((X(c0)-0.03,Y(h0)),X(c1-c0)+0.06,Y(h1-h0),boxstyle="round,pad=0.004,rounding_size=0.02",
                 fc="#3a3a3a",ec="#111",lw=0.8,alpha=0.9,zorder=2.5))

# jumpers
for j in S.JUMPERS:
    (c1,h1),(c2,h2)=j['a'],j['b']
    ax.plot([X(c1),X(c2)],[Y(h1),Y(h2)],color=rgbf(S.netrgb(j['net'])),lw=1.3,zorder=5,alpha=0.9)
# cuts
for (c,h) in S.CUTS:
    ax.plot([X(c)-0.028,X(c)+0.028],[Y(h)-0.028,Y(h)+0.028],color="red",lw=1.6,zorder=7)
    ax.plot([X(c)-0.028,X(c)+0.028],[Y(h)+0.028,Y(h)-0.028],color="red",lw=1.6,zorder=7)
# pads
for p in S.PADS:
    ax.add_patch(Circle((X(p['c']),Y(p['h'])),0.028,fc=rgbf(S.netrgb(p['net'])),ec="#222",lw=0.4,zorder=6))
# labels
for (c,h,t) in S.LABELS:
    ax.text(X(c),Y(h),t,ha="left",va="center",fontsize=6.5,fontweight="bold",color="#222",zorder=9)

ax.text(0.0,-0.02,"Mega Blastoise - single 24x55 stripboard  (red X=cut  dots=pads  lines=jumper wires)",
        ha="left",va="bottom",fontsize=9,fontweight="bold")
plt.tight_layout()
out="/home/alex/Code/mega_blastoise/hardware/stripboard/mega_blastoise_single_board_preview.png"
plt.savefig(out,dpi=130,bbox_inches="tight",facecolor="white"); print("saved",out)
