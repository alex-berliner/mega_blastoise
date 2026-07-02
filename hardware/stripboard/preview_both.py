"""Front (component side) and back (copper side) of the single board, side by
side in one image. Same verified model; the copper panel is the mirror."""
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, FancyBboxPatch
import gen_single_board as S

def rgbf(t): return tuple(v/255 for v in t)
NC, NH = S.NCOLS, S.NHOLES

def draw(ax, copper):
    def X(c): return (NC-1-c)*0.1 if copper else c*0.1
    def Y(h): return h*0.1
    ax.set_xlim(-0.45,2.75); ax.set_ylim(-0.12,5.75); ax.set_aspect('equal'); ax.axis('off')
    ax.invert_yaxis()
    bx = min(X(0),X(NC-1))
    ax.add_patch(FancyBboxPatch((bx-0.05,Y(1)-0.05),2.3+0.10,Y(NH-1)+0.10,
                 boxstyle="round,pad=0.01,rounding_size=0.05",
                 fc="#f0e0c8" if copper else "#f4f0e4",ec="#cbbf9a",lw=1.5,zorder=0))
    for c in range(NC):
        if copper:
            ax.add_patch(Rectangle((X(c)-0.028,Y(1)),0.056,Y(NH-1),fc="#da8a67",ec="#b06a4a",lw=0.3,alpha=0.85,zorder=0.5))
        else:
            ax.add_patch(Rectangle((X(c)-0.012,Y(1)),0.024,Y(NH-1),fc="#da8a67",ec="none",alpha=0.30,zorder=0.5))
        ax.text(X(c),Y(0.2),S.letter(c),ha="center",va="center",fontsize=5.5,color="#666")
    for h in range(5,NH+1,5):
        ax.text(bx-0.15,Y(h),str(h),ha="center",va="center",fontsize=5,color="#999")

    # components: solid on front, dashed ghosts on back
    style = dict(fc="none",ec="#667",lw=0.9,ls=(0,(3,3)),alpha=0.6) if copper else \
            dict(fc="#232838",ec="#11141f",lw=1.0,alpha=0.85)
    for (nm,c0,h0,c1,h1) in S.MODULES:
        x0=min(X(c0),X(c1)); w_=abs(X(c1)-X(c0))
        ax.add_patch(FancyBboxPatch((x0,Y(h0)),w_,Y(h1-h0),
                     boxstyle="round,pad=0.005,rounding_size=0.02",zorder=2.6,**style))
        if not copper and nm.startswith('OLED'):
            ax.add_patch(Rectangle((x0+0.07,Y(h0+3.0)),w_-0.14,Y(h1-h0-4.6),fc="#0a1a3a",ec="#3af",lw=0.7,zorder=2.7))
            ax.text(x0+w_/2,Y((h0+h1)/2+0.8),nm.split(' (')[0],ha="center",va="center",
                    fontsize=6,color="#7cf",fontweight="bold",zorder=2.8)
    swstyle = dict(fc="none",ec="#888",lw=0.7,ls=(0,(2,2)),alpha=0.6) if copper else \
              dict(fc="#3a3a3a",ec="#111",lw=0.8,alpha=0.9)
    for (c0,h0,c1,h1) in S.SW_BODIES:
        x0=min(X(c0),X(c1)); w_=abs(X(c1)-X(c0))
        ax.add_patch(FancyBboxPatch((x0-0.03,Y(h0)),w_+0.06,Y(h1-h0),
                     boxstyle="round,pad=0.004,rounding_size=0.02",zorder=2.5,**swstyle))
    px0=min(X(S.PC0-0.5),X(S.PC0+19.5)); pw=abs(X(S.PC0+19.5)-X(S.PC0-0.5))
    pstyle = dict(fc="none",ec="#556",lw=1.1,ls=(0,(4,3)),alpha=0.7) if copper else \
             dict(fc="#464650",ec="#28282d",lw=1.2,alpha=0.92)
    ax.add_patch(FancyBboxPatch((px0,Y(S.PTOP-0.7)),pw,Y(S.PBOT-S.PTOP+1.4),
                 boxstyle="round,pad=0.01,rounding_size=0.03",zorder=2,**pstyle))
    ax.text(px0+pw/2,Y((S.PTOP+S.PBOT)/2),
            "Pico (far side; USB right)" if copper else "Raspberry Pi Pico (USB left)",
            ha="center",va="center",fontsize=6.5,fontweight="bold",
            color="#556" if copper else "#eee",zorder=3)

    # copper-side work: wires, pads, cuts (shown on both, solid emphasis on back)
    for j in S.JUMPERS:
        (c1,h1),(c2,h2)=j['a'],j['b']
        ax.plot([X(c1),X(c2)],[Y(h1),Y(h2)],color=rgbf(S.netrgb(j['net'])),
                lw=1.4 if copper else 1.0,zorder=5,alpha=0.95 if copper else 0.75)
    for p in S.PADS:
        gray = p['net'].startswith('x')
        ax.add_patch(Circle((X(p['c']),Y(p['h'])),0.018 if gray else 0.024,
                     fc="#999" if gray else rgbf(S.netrgb(p['net'])),ec="#222",lw=0.4,zorder=6))
    for (c,h) in S.CUTS:
        s = 0.030 if copper else 0.024
        ax.plot([X(c)-s,X(c)+s],[Y(h)-s,Y(h)+s],color="red",lw=1.8 if copper else 1.3,zorder=7)
        ax.plot([X(c)-s,X(c)+s],[Y(h)+s,Y(h)-s],color="red",lw=1.8 if copper else 1.3,zorder=7)
    ax.set_title("BACK - copper side up (cut + solder here)" if copper else
                 "FRONT - component side (parts mount here)",
                 fontsize=10, fontweight="bold", pad=8)

fig,(axf,axb) = plt.subplots(1,2,figsize=(21,22))
draw(axf, copper=False)
draw(axb, copper=True)
fig.suptitle("Mega Blastoise single board rev 3 - front and back (mirror views of the same board)",
             fontsize=12, fontweight="bold", y=0.995)
plt.tight_layout()
out="/home/alex/Code/mega_blastoise/hardware/stripboard/mega_blastoise_single_board_both.png"
plt.savefig(out,dpi=120,bbox_inches="tight",facecolor="white"); print("saved",out)
