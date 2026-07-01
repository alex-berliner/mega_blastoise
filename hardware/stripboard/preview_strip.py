import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, FancyBboxPatch

# mirror gen_matrix_diy.py
VSPAN, HSPAN, COLPITCH = 0.3, 0.2, 0.4
def T(x, y): return (round(y - 0.3, 2), round(x - 2.0 + 0.5, 2))
ROWS = [('P1','M',4,'GP5',2.4),('P1','S',3,'GP7',1.8),
        ('P2','M',4,'GP8',1.2),('P2','S',3,'GP9',0.6)]
def node_of(by): return round(by - VSPAN, 2)
def xL(c): return round(2.4 + COLPITCH*c, 2)
def xgap(c): return round(2.4 + COLPITCH*c + 0.1, 2)
COLS = [(0,'GP10'),(1,'GP11'),(2,'GP12'),(3,'GP13')]

BX0,BY0,BX1,BY1 = 0.0,0.30,2.3,2.60

fig,ax = plt.subplots(figsize=(13,13))
ax.set_xlim(-0.3,2.6); ax.set_ylim(-0.05,2.95); ax.set_aspect('equal'); ax.axis('off')
ax.invert_yaxis()  # DIYLC screen coords (y down)

# board
ax.add_patch(FancyBboxPatch((BX0,BY0),BX1-BX0,BY1-BY0,boxstyle="round,pad=0.01,rounding_size=0.05",
             fc="#f4f0e4",ec="#cbbf9a",lw=2,zorder=0))
# VERTICAL copper strips (one per letter column) at every 0.1"
letters = "ABCDEFGHIJKLMNOPQRSTUVWX"
x=BX0; li=0
while x<=BX1+1e-6:
    ax.add_patch(Rectangle((x-0.012,BY0),0.024,BY1-BY0,fc="#da8a67",ec="none",alpha=0.35,zorder=0.5))
    if li < len(letters):
        ax.text(x,BY0-0.06,letters[li],ha="center",va="center",fontsize=7,color="#777")
    y=BY0
    while y<=BY1+1e-6:
        ax.add_patch(Circle((x,y),0.013,fc="#b98a5f",ec="none",zorder=0.6))
        y=round(y+0.1,2)
    x=round(x+0.1,2); li+=1
# hole numbers down the side
h=1; y=round(BY0+0.1,2)
while y<=BY1-0.05:
    ax.text(BX0-0.10,y,str(h),ha="center",va="center",fontsize=6,color="#999")
    y=round(y+0.1,2); h+=1

# buttons
for (p,k,nc,gp,by) in ROWS:
    ny=node_of(by)
    for c in range(nc):
        rx,ryv=T(xL(c),by)                    # green (row leg, bus)
        cx,cyv=T(round(xL(c)+HSPAN,2),ny)     # blue  (col leg, node)
        x0=min(rx,cx); y0=min(ryv,cyv)
        ax.add_patch(FancyBboxPatch((x0-0.06,y0-0.06),abs(rx-cx)+0.12,abs(ryv-cyv)+0.12,
                     boxstyle="round,pad=0.005,rounding_size=0.03",fc="#3a3a3a",ec="#111",lw=1,alpha=0.85,zorder=3))
        for lx in (rx,cx):
            for ly in (ryv,cyv):
                ax.add_patch(Circle((lx,ly),0.026,fc="#999",ec="#333",lw=0.5,zorder=4))
        ax.add_patch(Circle((rx,ryv),0.030,fc="#00b000",ec="#024",lw=0.6,zorder=5))
        ax.add_patch(Circle((cx,cyv),0.030,fc="#0060d0",ec="#012",lw=0.6,zorder=5))
        lx,ly=T(round(xL(c)+HSPAN/2,2),round((by+ny)/2,2))
        ax.text(lx,ly,f"{k}{c+1}",color="white",ha="center",va="center",fontsize=7,fontweight="bold",zorder=6)

# cuts (red X) on node strips
for (p,k,nc,gp,by) in ROWS:
    ny=node_of(by)
    for c in range(nc-1):
        cx,cyv=T(round(xL(c)+HSPAN+0.1,2),ny)
        ax.plot([cx-0.03,cx+0.03],[cyv-0.03,cyv+0.03],color="red",lw=2.2,zorder=7)
        ax.plot([cx-0.03,cx+0.03],[cyv+0.03,cyv-0.03],color="red",lw=2.2,zorder=7)

# column jumpers (now horizontal in board frame)
for (c,gp) in COLS:
    nodes=sorted(node_of(by) for (p,k,nc,g,by) in ROWS if c<nc)
    pts=[T(xgap(c),n) for n in nodes]
    xs=[q[0] for q in pts]; ys=[q[1] for q in pts]
    ax.plot(xs,ys,color="#0050c0",lw=2.4,zorder=5)
    for q in pts:
        ax.add_patch(Circle(q,0.028,fc="#0050c0",ec="none",zorder=6))

# header pads
for (p,k,nc,gp,by) in ROWS:
    hx,hy=T(2.1,by)
    ax.add_patch(Circle((hx,hy),0.038,fc="#e06000",ec="#500",zorder=6))
    ax.text(hx,hy-0.10,gp,ha="center",va="center",fontsize=8,fontweight="bold",color="#b04000",zorder=7)
for (c,gp) in COLS:
    nodes=sorted(node_of(by) for (p,k,nc,g,by) in ROWS if c<nc)
    hx,hy=T(xgap(c),nodes[0])
    ax.add_patch(Circle((hx,hy),0.038,fc="#e06000",ec="#500",zorder=6))
    ax.text(hx-0.09,hy,gp,ha="right",va="center",fontsize=7.5,fontweight="bold",color="#b04000",zorder=7)

ax.text(0.0,-0.02,"Mega Blastoise 4x4 matrix - stripboard (vertical strips, A-X down cols)   "
        "green=row leg  blue=col leg  X=track cut  line=col jumper  orange=Pico wire",
        ha="left",va="bottom",fontsize=8.5,fontweight="bold")
plt.tight_layout()
out="/home/alex/Code/mega_blastoise/hardware/stripboard/mega_blastoise_matrix_preview.png"
plt.savefig(out,dpi=140,bbox_inches="tight",facecolor="white"); print("saved",out)
