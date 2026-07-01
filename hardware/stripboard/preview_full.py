import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, FancyBboxPatch
import gen_matrix_full_diy as F   # imports gen_matrix_diy as F.g and builds all geometry
g = F.g
T, ROWS, COLS = g.T, g.ROWS, g.COLS
node_of, xL, xgap, HSPAN = g.node_of, g.xL, g.xgap, g.HSPAN

BX0,BY0,BX1,BY1 = 0.0,0.30,2.3,2.60
fig,ax = plt.subplots(figsize=(13,20))
ax.set_xlim(-0.35,2.7); ax.set_ylim(-0.05,5.35); ax.set_aspect('equal'); ax.axis('off')
ax.invert_yaxis()

# board + vertical strips
ax.add_patch(FancyBboxPatch((BX0,BY0),BX1-BX0,BY1-BY0,boxstyle="round,pad=0.01,rounding_size=0.05",
             fc="#f4f0e4",ec="#cbbf9a",lw=2,zorder=0))
letters="ABCDEFGHIJKLMNOPQRSTUVWX"; x=BX0; li=0
while x<=BX1+1e-6:
    ax.add_patch(Rectangle((x-0.012,BY0),0.024,BY1-BY0,fc="#da8a67",ec="none",alpha=0.35,zorder=0.5))
    if li<len(letters): ax.text(x,BY0-0.06,letters[li],ha="center",va="center",fontsize=7,color="#777")
    y=BY0
    while y<=BY1+1e-6:
        ax.add_patch(Circle((x,y),0.013,fc="#b98a5f",ec="none",zorder=0.6)); y=round(y+0.1,2)
    x=round(x+0.1,2); li+=1

# buttons (bodies + legs)
for (p,k,nc,gp,by) in ROWS:
    ny=node_of(by)
    for c in range(nc):
        rx,ryv=T(xL(c),by); cx,cyv=T(round(xL(c)+HSPAN,2),ny)
        x0=min(rx,cx); y0=min(ryv,cyv)
        ax.add_patch(FancyBboxPatch((x0-0.06,y0-0.06),abs(rx-cx)+0.12,abs(ryv-cyv)+0.12,
                     boxstyle="round,pad=0.005,rounding_size=0.03",fc="#3a3a3a",ec="#111",lw=1,alpha=0.9,zorder=3))
        for lx in (rx,cx):
            for ly in (ryv,cyv): ax.add_patch(Circle((lx,ly),0.026,fc="#999",ec="#333",lw=0.5,zorder=4))
        ax.add_patch(Circle((rx,ryv),0.030,fc="#00b000",ec="#024",lw=0.6,zorder=5))
        ax.add_patch(Circle((cx,cyv),0.030,fc="#0060d0",ec="#012",lw=0.6,zorder=5))
        lx,ly=T(round(xL(c)+HSPAN/2,2),round((by+ny)/2,2))
        ax.text(lx,ly,f"{k}{c+1}",color="white",ha="center",va="center",fontsize=7,fontweight="bold",zorder=6)

# cuts + col jumpers
for (p,k,nc,gp,by) in ROWS:
    ny=node_of(by)
    for c in range(nc-1):
        cx,cyv=T(round(xL(c)+HSPAN+0.1,2),ny)
        ax.plot([cx-0.03,cx+0.03],[cyv-0.03,cyv+0.03],color="red",lw=2.2,zorder=7)
        ax.plot([cx-0.03,cx+0.03],[cyv+0.03,cyv-0.03],color="red",lw=2.2,zorder=7)
for (c,gp) in COLS:
    nodes=sorted(node_of(by) for (p,k,nc,gg,by) in ROWS if c<nc)
    pts=[T(xgap(c),n) for n in nodes]
    ax.plot([q[0] for q in pts],[q[1] for q in pts],color="#0050c0",lw=2.2,zorder=5)
    for q in pts: ax.add_patch(Circle(q,0.026,fc="#0050c0",ec="none",zorder=6))

# header pads
for gp,(hx,hy) in F.HEADER.items():
    ax.add_patch(Circle((hx,hy),0.038,fc="#e06000",ec="#500",zorder=6))

# ribbon wires (through the 4 control points)
for (gp,rgb,route) in F.WIRES:
    xs=[q[0] for q in route]; ys=[q[1] for q in route]
    ax.plot(xs,ys,color=tuple(v/255 for v in rgb),lw=2.0,zorder=8,solid_capstyle='round')

# Pico (40-pin DIL) below the board
px0,py0,row=F.PICO_X0,F.PICO_Y0,F.PICO_ROW
ax.add_patch(FancyBboxPatch((px0-0.10,py0-0.10),row+0.20,1.9+0.20,boxstyle="round,pad=0.01,rounding_size=0.04",
             fc="#464650",ec="#28282d",lw=1.5,zorder=7))
ax.text(px0+row/2,py0-0.02,"Raspberry Pi Pico",color="#e6e6e6",ha="center",va="center",fontsize=8,fontweight="bold",zorder=9,rotation=90)
used={F.GP_PIN[gp]:gp for gp in F.GP_PIN}
for side,xx in ((0,px0),(1,px0+row)):
    for i in range(20):
        pin=i+1 if side==0 else 40-i
        yy=round(py0+0.1*i,2)
        if side==0 and pin in used:
            gp=used[pin]; rgb=tuple(v/255 for v in F.WIRE_RGB[gp])
            ax.add_patch(Circle((xx,yy),0.032,fc=rgb,ec="#111",lw=0.6,zorder=9))
            ax.text(xx-0.06,yy,f"{gp} p{pin}",ha="right",va="center",fontsize=6.2,color=rgb,fontweight="bold",zorder=9)
        else:
            ax.add_patch(Circle((xx,yy),0.024,fc="#999",ec="#333",lw=0.4,zorder=8))

# parts list
for i,line in enumerate(F.PARTS):
    ax.text(1.72,round(3.30+0.20*i,2),line,ha="left",va="center",
            fontsize=8.5 if i==0 else 7.5,fontweight="bold" if i==0 else "normal",color="#222",zorder=9)

ax.text(0.0,-0.02,"Mega Blastoise - full assembly: matrix + Pico + 8-way ribbon (colours = wires to Pico pins)",
        ha="left",va="bottom",fontsize=9,fontweight="bold")
plt.tight_layout()
out="/home/alex/Code/mega_blastoise/hardware/stripboard/mega_blastoise_matrix_full_preview.png"
plt.savefig(out,dpi=130,bbox_inches="tight",facecolor="white"); print("saved",out)
