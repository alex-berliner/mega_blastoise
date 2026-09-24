"""A small grid router for the connections Freerouting leaves open.

Freerouting routes the board as a whole and can give up on a net when the area
around a pin is crowded. This one routes a single connection at a time, pad to
pad, on the top and bottom layers with vias, by A* over a 0.05 mm grid:

  * every copper item of another net is an obstacle, grown by the clearance
    plus half the track width (for the track grid) or the via radius (for the
    via grid); drill holes too
  * rule areas count as they say (no tracks, no vias); the core's small copper
    zones, route.py's fences, the membrane contacts and the board's cut-outs
    count as obstacles; a pad that asks for more clearance than the net class
    (a fiducial) gets it, and a via keeps its drill clear of every other hole
  * the grid path is straightened (a segment replaces a run of cells when the
    straight line stays clear) and every segment is checked again against the
    real shapes before anything is added

It is slow in Python and meant for a handful of connections, not a board.
"""
import heapq
import math

import pcbnew

GRID = 0.05
MARGIN = 0.04            # a straight line between two free cell centres can pass this much closer


def mm(v):
    return pcbnew.FromMM(v)


def V(x, y):
    return pcbnew.VECTOR2I(mm(x), mm(y))


def xy(v):
    return pcbnew.ToMM(v.x), pcbnew.ToMM(v.y)


def seg_dist(p, a, b):
    L2 = (b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2
    if L2 < 1e-12:
        return math.dist(p, a)
    u = max(0.0, min(1.0, ((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / L2))
    return math.dist(p, (a[0] + u * (b[0] - a[0]), a[1] + u * (b[1] - a[1])))


class Maze:
    def __init__(self, board, net, box, layers, width, clear, via_d, via_drill, fences=(), blocks=(), edge=None,
                 ignore=(), discs=()):
        self.board, self.net, self.layers = board, net, list(layers)
        self.width, self.clear, self.via_d, self.via_drill = width, clear, via_d, via_drill
        self.x0, self.y0 = box[0], box[1]
        self.nx = int((box[2] - box[0]) / GRID) + 1
        self.ny = int((box[3] - box[1]) / GRID) + 1
        self.track = {l: bytearray(self.nx * self.ny) for l in self.layers}
        self.via = bytearray(self.nx * self.ny)
        self.shapes = {l: [] for l in self.layers}   # (item, layer) for the final check
        self.holes = []
        if edge:
            self._block_outside(edge)
        grow_t = clear + width / 2 + MARGIN
        grow_v = clear + via_d / 2 + MARGIN
        for fp in board.GetFootprints():
            for p in fp.Pads():
                if p.GetAttribute() in (pcbnew.PAD_ATTRIB_PTH, pcbnew.PAD_ATTRIB_NPTH) and p.GetDrillSize().x > 0:
                    self.holes.append((xy(p.GetPosition()), pcbnew.ToMM(max(p.GetDrillSize().x, p.GetDrillSize().y)) / 2))
                if p.GetNetname() == net and p.GetAttribute() != pcbnew.PAD_ATTRIB_NPTH:
                    continue
                # a pad may ask for more than the net class (fiducials: 0.6 mm)
                extra = max(0.0, pcbnew.ToMM(p.GetLocalClearance() or 0) - clear)
                self._item(p, grow_t + extra, grow_v + extra)
        for t in board.GetTracks():
            if t.GetClass() == "PCB_VIA":
                # drill to drill, whatever the net
                self.holes.append((xy(t.GetPosition()), pcbnew.ToMM(t.GetDrillValue()) / 2))
            if t.GetNetname() == net or t.m_Uuid.AsString() in ignore:
                continue
            self._item(t, grow_t, grow_v)
        for z in board.Zones():
            if z.GetIsRuleArea():
                for l in self.layers:
                    if not z.IsOnLayer(l):
                        continue
                    if z.GetDoNotAllowTracks():
                        self._zone(z, l, self.track[l], grow_t)
                    if z.GetDoNotAllowVias():
                        self._zone(z, l, self.via, grow_v)
            elif z.GetNetname() != net and z.GetAssignedPriority() >= 10:
                # the core's small copper zones (copied with priority + 10)
                for l in self.layers:
                    if z.IsOnLayer(l):
                        self._zone(z, l, self.track[l], grow_t)
                        self._zone(z, l, self.via, grow_v)
        for f in fences:                         # route.py's fences: top layer (and so no vias)
            self._rect(f, self.track[self.layers[0]], grow_t)
            self._rect(f, self.via, grow_v)
        for r in blocks:                         # cut-outs: every layer
            for l in self.layers:
                self._rect(r, self.track[l], grow_t)
            self._rect(r, self.via, grow_v)
        for c, r in discs:                       # membrane contacts: nothing but the combs, top layer
            self._capsule(c, c, r + width / 2, self.track[self.layers[0]])
            self._capsule(c, c, r + via_d / 2, self.via)
        # via drill to any other hole
        for (hx, hy), hr in self.holes:
            self._disc((hx, hy), hr + self.via_drill / 2 + 0.25 + MARGIN, self.via)

    # ---- rasterising -------------------------------------------------------------
    def cell(self, x, y):
        return int(round((x - self.x0) / GRID)), int(round((y - self.y0) / GRID))

    def pos(self, i, j):
        return self.x0 + i * GRID, self.y0 + j * GRID

    def _cells(self, bb, grow):
        i0, j0 = self.cell(bb[0] - grow, bb[1] - grow)
        i1, j1 = self.cell(bb[2] + grow, bb[3] + grow)
        for j in range(max(j0, 0), min(j1, self.ny - 1) + 1):
            for i in range(max(i0, 0), min(i1, self.nx - 1) + 1):
                yield i, j

    def _item(self, item, grow_t, grow_v):
        b = item.GetBoundingBox()
        bb = (pcbnew.ToMM(b.GetX()), pcbnew.ToMM(b.GetY()), pcbnew.ToMM(b.GetRight()), pcbnew.ToMM(b.GetBottom()))
        if bb[2] < self.x0 - 2 or bb[1] > self.y0 + self.ny * GRID + 2 or bb[0] > self.x0 + self.nx * GRID + 2 \
                or bb[3] < self.y0 - 2:
            return
        on = [l for l in self.layers if item.IsOnLayer(l)]
        for l in on:
            self.shapes[l].append((item, bb))
        if isinstance(item, pcbnew.PCB_VIA):
            c, r = xy(item.GetPosition()), pcbnew.ToMM(item.GetWidth(pcbnew.F_Cu)) / 2
            for l in self.layers:
                self._capsule(c, c, r + grow_t, self.track[l])
            self._capsule(c, c, r + grow_v, self.via)
            return
        if isinstance(item, pcbnew.PCB_TRACK):
            # long diagonal tracks have huge bounding boxes: mark along the track instead
            a, b, r = xy(item.GetStart()), xy(item.GetEnd()), pcbnew.ToMM(item.GetWidth()) / 2
            for l in on:
                self._capsule(a, b, r + grow_t, self.track[l])
            if on:
                self._capsule(a, b, r + grow_v, self.via)
            return
        for l in on:
            grid = self.track[l]
            for i, j in self._cells(bb, grow_t):
                k = j * self.nx + i
                if not grid[k] and self._hit(item, l, self.pos(i, j), grow_t):
                    grid[k] = 1
        if on:
            for i, j in self._cells(bb, grow_v):
                k = j * self.nx + i
                if not self.via[k] and any(self._hit(item, l, self.pos(i, j), grow_v) for l in on):
                    self.via[k] = 1

    def _capsule(self, a, b, r, grid):
        """Mark the cells within r of segment a-b, row by row."""
        (ax, ay), (bx, by) = a, b
        j0 = max(0, int(math.floor((min(ay, by) - r - self.y0) / GRID)))
        j1 = min(self.ny - 1, int(math.ceil((max(ay, by) + r - self.y0) / GRID)))
        dy = by - ay
        for j in range(j0, j1 + 1):
            y = self.y0 + j * GRID
            if abs(dy) < 1e-9:
                if abs(y - ay) > r:
                    continue
                lo, hi = min(ax, bx), max(ax, bx)
            else:
                t0, t1 = sorted(((y - r - ay) / dy, (y + r - ay) / dy))
                t0, t1 = max(0.0, t0), min(1.0, t1)
                if t0 > t1:
                    continue
                xa, xb = ax + (bx - ax) * t0, ax + (bx - ax) * t1
                lo, hi = min(xa, xb), max(xa, xb)
            i0 = max(0, int(math.floor((lo - r - self.x0) / GRID)))
            i1 = min(self.nx - 1, int(math.ceil((hi + r - self.x0) / GRID)))
            row = j * self.nx
            for i in range(i0, i1 + 1):
                if not grid[row + i] and seg_dist((self.x0 + i * GRID, y), a, b) <= r:
                    grid[row + i] = 1

    @staticmethod
    def _hit(item, layer, p, grow):
        # pads here have the same shape on every copper layer
        return item.HitTest(V(*p), mm(grow))

    def _zone(self, z, layer, grid, grow):
        o = z.Outline()
        b = o.BBox()
        bb = (pcbnew.ToMM(b.GetX()), pcbnew.ToMM(b.GetY()), pcbnew.ToMM(b.GetRight()), pcbnew.ToMM(b.GetBottom()))
        for i, j in self._cells(bb, grow):
            p = V(*self.pos(i, j))
            if o.Contains(p) or o.SquaredDistance(p) <= mm(grow) ** 2:
                grid[j * self.nx + i] = 1

    def _rect(self, r, grid, grow):
        for i, j in self._cells(r, grow):
            grid[j * self.nx + i] = 1

    def _disc(self, c, r, grid):
        for i, j in self._cells((c[0], c[1], c[0], c[1]), r):
            if math.dist(self.pos(i, j), c) < r:
                grid[j * self.nx + i] = 1

    def _block_outside(self, edge):
        x0, y0, x1, y1 = edge
        for j in range(self.ny):
            for i in range(self.nx):
                x, y = self.pos(i, j)
                if not (x0 <= x <= x1 and y0 <= y <= y1):
                    for l in self.layers:
                        self.track[l][j * self.nx + i] = 1
                    self.via[j * self.nx + i] = 1

    # ---- search --------------------------------------------------------------------
    def route(self, a, la, b, lb, via_cost=2.0):
        """a, b: (x, y) pad centres on layers la, lb. Returns [(layer, [(x, y), ...]), ...]
        runs joined by vias, or None."""
        start = self.cell(*a)
        goal = self.cell(*b)
        L = self.layers
        si, gi = L.index(la), L.index(lb)
        steps = [(1, 0, 1.0), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0),
                 (1, 1, 1.414), (1, -1, 1.414), (-1, 1, 1.414), (-1, -1, 1.414)]
        vc = via_cost / GRID

        def h(i, j, l):
            dx, dy = abs(i - goal[0]), abs(j - goal[1])
            return max(dx, dy) + 0.414 * min(dx, dy) + (vc if l != gi else 0)
        s = (si, start[0], start[1])
        # the pad's own cells are free to leave from even when a neighbour's grown shape reaches them
        own = {s, (gi, goal[0], goal[1])}
        dist = {s: 0.0}
        prev = {}
        heap = [(h(start[0], start[1], si), 0.0, s)]
        while heap:
            f, g, node = heapq.heappop(heap)
            if g > dist.get(node, 1e18):
                continue
            l, i, j = node
            if (i, j) == goal and l == gi:
                break
            for di, dj, c in steps:
                ni, nj = i + di, j + dj
                if not (0 <= ni < self.nx and 0 <= nj < self.ny):
                    continue
                nxt = (l, ni, nj)
                if self.track[L[l]][nj * self.nx + ni] and nxt not in own:
                    continue
                ng = g + c
                if ng < dist.get(nxt, 1e18):
                    dist[nxt] = ng
                    prev[nxt] = node
                    heapq.heappush(heap, (ng + h(ni, nj, l), ng, nxt))
            if not self.via[j * self.nx + i]:
                for nl in range(len(L)):
                    if nl == l or self.track[L[nl]][j * self.nx + i]:
                        continue
                    nxt = (nl, i, j)
                    ng = g + vc
                    if ng < dist.get(nxt, 1e18):
                        dist[nxt] = ng
                        prev[nxt] = node
                        heapq.heappush(heap, (ng + h(i, j, nl), ng, nxt))
        end = (gi, goal[0], goal[1])
        if end not in prev and end != s:
            return None
        path = [end]
        while path[-1] != s:
            path.append(prev[path[-1]])
        path.reverse()
        runs = []
        for l, i, j in path:
            if not runs or runs[-1][0] != L[l]:
                runs.append((L[l], []))
            runs[-1][1].append(self.pos(i, j))
        runs[0][1][0] = a
        runs[-1][1][-1] = b
        return [(l, self._straighten(l, pts)) for l, pts in runs]

    def near(self, layer, p, q, grow):
        x0, x1 = min(p[0], q[0]) - grow - 0.1, max(p[0], q[0]) + grow + 0.1
        y0, y1 = min(p[1], q[1]) - grow - 0.1, max(p[1], q[1]) + grow + 0.1
        return [it for it, bb in self.shapes[layer] if bb[2] >= x0 and bb[0] <= x1 and bb[3] >= y0 and bb[1] <= y1]

    def _clear_line(self, layer, p, q):
        n = max(1, int(math.dist(p, q) / (GRID / 2)))
        grid = self.track[layer]
        for k in range(n + 1):
            x = p[0] + (q[0] - p[0]) * k / n
            y = p[1] + (q[1] - p[1]) * k / n
            i, j = self.cell(x, y)
            if grid[j * self.nx + i] and k not in (0, n):
                return False
        return True

    def _straighten(self, layer, pts):
        """Longest straight segments that stay clear on the grid and against the real shapes;
        one cell at a time is always clear (the grid's margin covers it)."""
        out = [pts[0]]
        i = 0
        while i < len(pts) - 1:
            j = len(pts) - 1
            while j > i + 1 and not (self._clear_line(layer, pts[i], pts[j]) and self.segment_ok(layer, pts[i], pts[j])):
                j -= 1
            out.append(pts[j])
            i = j
        return out

    # ---- checking and adding -------------------------------------------------------------
    def segment_ok(self, layer, p, q):
        """Exact check of a segment against the real shapes of other nets."""
        grow = self.clear + self.width / 2
        items = self.near(layer, p, q, grow)
        n = max(1, int(math.dist(p, q) / 0.02))
        for k in range(n + 1):
            x = p[0] + (q[0] - p[0]) * k / n
            y = p[1] + (q[1] - p[1]) * k / n
            for item in items:
                if self._hit(item, layer, (x, y), grow + 0.005):
                    return False
        return True

    def via_ok(self, p):
        grow = self.clear + self.via_d / 2
        for l in self.layers:
            for item in self.near(l, p, p, grow):
                if self._hit(item, l, p, grow + 0.005):
                    return False
        for c, r in self.holes:
            if math.dist(c, p) < r + self.via_drill / 2 + 0.25:
                return False
        return True
