"""Generate source-derived parity fixtures without installing or running Rust.

Reference: eabab1e, note_atmosphere.rs. Tables are read verbatim from Git;
formulas below are an independent Python evaluation of that source, not calls
into the Go implementation. Run from the repository root with Python 3.
These fixtures establish numeric parity, not GPUI/MyGo pixel equivalence.
"""
import json
import math
from pathlib import Path
import re
import struct
import subprocess

SOURCE = 'source/crates/locus-workspace/src/workspace/note_atmosphere.rs'
source = subprocess.check_output(['git', 'show', f'eabab1e:{SOURCE}'], text=True)
slots = re.findall(r'Slot\(([^)]+)\)', source.split('const LAYOUTS:')[1].split('const FAMILIES:')[0])
slots = [list(map(float, s.split(','))) for s in slots]
layouts = [slots[i:i+6] for i in range(0, 36, 6)]
kinds = {'Disk': 0, 'Ring': 1, 'RoundedRect': 2, 'Arc': 3, 'Triangle': 4}
family = [kinds[k] for k in re.findall(r'Kind::(\w+)', source.split('const FAMILIES:')[1].split('pub(super) fn')[0])]
families = [family[i:i+6] for i in range(0, 60, 6)]

def f(v): return struct.unpack('f', struct.pack('f', v))[0]
def rnd(v, digits=2):
    p = 10 ** digits
    return math.copysign(math.floor(abs(v) * p + .5), v) / p

def clamp(v, lo, hi): return min(hi, max(lo, v))
def hash_title(title):
    h = 2166136261
    for c in title:
        c = ord(c)
        if c > 65535: c = 0xd800 + ((c - 65536) >> 10)
        h = ((h ^ c) * 16777619) & 0xffffffff
    return h

def mix(h, salt):
    n = h ^ salt
    n = ((n ^ (n >> 16)) * 2246822507) & 0xffffffff
    n = ((n ^ (n >> 13)) * 3266489909) & 0xffffffff
    return n ^ (n >> 16)

def units(local): return lambda salt: mix(local, salt) / 4294967296

def color(h, s, l, a): return dict(H=f(h/360), S=f(s/100), L=f(l/100), A=f(a))
def opacity(c, a): return dict(c, A=f(c['A'] * f(a)))
def tone(h, dark, fill, salt):
    if fill: return color(h, (32 if dark else 36) + salt % 10, (52+salt%10) if dark else (56+salt%12), 1)
    return color(h, (28 if dark else 32) + salt % 8, (68 if dark else 40) + salt % 8, 1)

def backdrop(seed, hue, dark):
    local = mix(seed, 0xbd01)
    u = units(local)
    stroke = opacity(tone(hue, dark, False, mix(local, 3) % 16), rnd((.035 if dark else .06) + u(2)*(.025 if dark else .04), 3))
    spacing = f(rnd(18+u(4)*16))
    p = dict(Kind=mix(local, 1)%7, Spacing=spacing, Rotate=f(rnd(-8+u(5)*16)), OffsetX=f(rnd(u(6)*spacing)), OffsetY=f(rnd(u(7)*spacing)), StrokeWidth=f(rnd(.5+u(8)*.45)), Radius=0, Amplitude=0, Color=stroke)
    k = p['Kind']
    if k == 0: return {}
    if k == 1: p['Color'] = opacity(stroke, .72)
    if k == 3: p['Radius'] = f(rnd(.55+u(9)*.55))
    if k == 4:
        p['Spacing'] = f(rnd(spacing*1.08))
        p['Rotate'] = f(rnd(p['Rotate']+24))
    if k == 5:
        p['Spacing'] = f(rnd(spacing*1.16))
        p['Rotate'] = f(rnd(p['Rotate']*.45))
        p['StrokeWidth'] = f(rnd(p['StrokeWidth']*.9))
        p['Color'] = opacity(stroke, .78)
    if k == 6:
        p['Spacing'] = f(rnd(spacing*1.24))
        p['Rotate'] = f(rnd(p['Rotate']*.32))
        p['StrokeWidth'] = f(rnd(p['StrokeWidth']*.92))
        p['Color'] = opacity(stroke, .84)
        p['Amplitude'] = f(rnd(clamp(p['Spacing']*(.11+u(10)*.05), 2.4, 5.8)))
    return p

def margins(u, kind, prominence):
    angle = u(34)*math.tau
    if kind == 3:
        scale = 8+u(36)*6
        return abs(math.cos(angle)*scale)*.65, abs(math.sin(angle)*scale*.68)*.65
    scale = 16+(1-clamp(prominence, .35, 1))*11+u(29)*4
    sway = angle+math.pi*(.72+u(35)*.56)
    size = scale*(.7+u(36)*.24)
    return max(abs(math.cos(angle)*scale), abs(math.cos(sway)*size))*.65, max(abs(math.sin(angle)*scale*.68), abs(math.sin(sway)*size*.68))*.65

def shape(root, index, slot, kind, hue, dark):
    local = mix(root, 20+index*11)
    u = units(local)
    prominence = clamp(slot[2]*(.88+u(1)*.24), .34, 1.08)
    alpha = rnd((.085 if dark else .065)+prominence*.035+u(2)*(.045 if dark else .035), 3)
    hue = clamp(hue+[-9, 6, -4, 10, 2, -7][index], 190, 250)
    fill = opacity(tone(hue, dark, True, mix(local, 3)%20), alpha)
    stroke = tone(hue, dark, False, mix(local, 4)%20)
    rawx, rawy = slot[0]+(u(5)*2-1)*slot[3], slot[1]+(u(6)*2-1)*slot[4]
    mx, my = margins(u, kind, prominence)
    def center(extent):
        return f(rnd(clamp(rawx, 132+extent+mx, 356-extent-mx))), f(rnd(clamp(rawy, 4+extent+my, 96-extent-my)))
    s = dict(Kind=kind, Color=fill)
    if kind == 0:
        radius = rnd((17+u(7)*11)*prominence)
        s['X'], s['Y'] = center(radius)
        s['Radius'] = f(radius)
    if kind == 1:
        radius = rnd((17+u(7)*10)*prominence)
        width = rnd(1.15+u(8)*1.45*prominence)
        s['X'], s['Y'] = center(radius+width)
        s.update(Radius=f(radius), StrokeWidth=f(width), Color=opacity(stroke, alpha+.018))
    if kind == 2:
        height = rnd((18+u(7)*8)*prominence)
        width = rnd(height*(1.45+u(8)*.55))
        x, y = center(math.hypot(width, height)/2)
        s.update(X=f(rnd(x-width/2)), Y=f(rnd(y-height/2)), Width=f(width), Height=f(height), Radius=f(rnd(min(height*.22, (4+u(9)*3)*prominence))), Rotate=f(rnd(-14+u(10)*28)))
    if kind == 3:
        radius = rnd((18+u(7)*10)*prominence)
        width = rnd(1.2+u(10)*1.8*prominence)
        s['X'], s['Y'] = center(radius+width)
        s.update(Radius=f(radius), StrokeWidth=f(width), Start=f(rnd(u(8)*360, 1)), Sweep=f(rnd(105+u(9)*105, 1)), Color=opacity(stroke, alpha+.024))
    if kind == 4:
        size = rnd((20+u(7)*11)*prominence)
        x, y = center(size)
        angle = u(8)*math.tau
        s['Points'] = [dict(X=f(rnd(x+math.cos(angle+math.tau*i/3)*size)), Y=f(rnd(y+math.sin(angle+math.tau*i/3)*size))) for i in range(3)]
    return s

def fixture(title, dark):
    seed = hash_title(title)
    hue = rnd(190+units(seed)(1)*60, 1)
    root = mix(seed, 0x5a9e)
    layout, family = layouts[mix(root, 2)%6], families[mix(root, 3)%10]
    return dict(Title=title, Dark=dark, Seed=seed,
        Wash=color(hue, 36 if dark else 40, 48 if dark else 62, .16 if dark else .12),
        Panel=color(hue, 34 if dark else 42, 28 if dark else 90, .42 if dark else .58),
        Backdrop=backdrop(seed, hue, dark),
        Shapes=[shape(root, i, layout[i], family[i], hue, dark) for i in range(4+mix(root, 1)%3)])

# Cover every layout, family and backdrop, plus empty, CJK and astral titles.
titles = ['Locus', '', '中文文章', '🌌 Notes']
seen = set()
def coverage(title):
    seed = hash_title(title)
    root = mix(seed, 0x5a9e)
    return {('layout', mix(root, 2)%6), ('family', mix(root, 3)%10), ('backdrop', mix(mix(seed, 0xbd01), 1)%7)}
for title in titles: seen |= coverage(title)
for i in range(1000):
    title = f'Article {i}'
    if coverage(title) - seen:
        titles.append(title)
        seen |= coverage(title)
    if len(seen) == 23: break
assert len(seen) == 23
fixtures = [fixture(title, dark) for title in titles for dark in [False, True]]
output = Path('source/cmd/locus/testdata/atmosphere_reference.json')
output.write_text('[\n' + ',\n'.join(json.dumps(case, ensure_ascii=False, separators=(',', ':')) for case in fixtures) + '\n]\n')
print(f'Wrote {len(fixtures)} fixtures covering all layouts, families and backdrops')
