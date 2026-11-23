# Subtle self-made SFX bed for the flowmaid LinkedIn video.
# Reads events.json (written by render.js from the same timeline as the picture) and writes sfx.wav
# (48 kHz stereo, 16-bit) at a FIXED master gain; loudness is set later at mux time.
import numpy as np, wave, json, sys
SR = 48000
EV = json.load(open('events.json'))
DUR = float(sys.argv[1]) if len(sys.argv) > 1 else 34.5
rng = np.random.default_rng(7)
N = int(SR * DUR); L = np.zeros(N); R = np.zeros(N)

def tailfade(sig, ms=60):
    n = min(len(sig), int(SR * ms / 1000))
    if n > 1: sig[-n:] *= 0.5 * (1 + np.cos(np.linspace(0, np.pi, n)))
    return sig
def add(sig, t, gain=1.0, pan=0.0):
    sig = tailfade(sig.copy())
    i = int(round(t * SR)); j = min(N, i + len(sig))
    if j <= i or i < 0: return
    s = sig[:j - i] * gain
    L[i:j] += s * np.sqrt(0.5 * (1 - pan)); R[i:j] += s * np.sqrt(0.5 * (1 + pan))
def env(n, a, d):
    t = np.arange(n) / SR
    return np.minimum(1, t / max(a, 1e-4)) * np.exp(-t / d)
def onepole(x, cut):
    cut = np.broadcast_to(cut, x.shape)
    a = np.exp(-2 * np.pi * cut / SR); y = np.zeros_like(x); acc = 0.0
    for k in range(len(x)):
        acc = (1 - a[k]) * x[k] + a[k] * acc; y[k] = acc
    return y
def click(f=3200, d=0.012):
    n = int(0.06 * SR); t = np.arange(n) / SR
    noise = np.convolve(rng.standard_normal(n), np.ones(6) / 6, 'same')
    return (0.6 * noise + 0.4 * np.sin(2 * np.pi * f * t)) * env(n, 0.0007, d)
def pop(f=660, d=0.09, length=0.4):
    n = int(length * SR); t = np.arange(n) / SR
    fs = f * (1 + 0.5 * np.exp(-t / 0.012)); ph = 2 * np.pi * np.cumsum(fs) / SR
    return (np.sin(ph) + 0.25 * np.sin(2 * ph)) * env(n, 0.002, d)
def whoosh(dur=0.7):
    # darker than before: two cascaded one-pole low-passes, sweep up to ~2.4 kHz and back
    n = int(dur * SR); t = np.arange(n) / SR
    cut = 300 + 2100 * np.sin(np.pi * np.clip(t / dur, 0, 1)) ** 1.5
    y = onepole(onepole(rng.standard_normal(n), cut), cut)
    return y * np.sin(np.pi * np.clip(t / dur, 0, 1)) ** 2 * 6.0
def thump(f=70, d=0.25):
    n = int(1.4 * SR); t = np.arange(n) / SR
    fs = f * (1 + 1.5 * np.exp(-t / 0.03)); ph = 2 * np.pi * np.cumsum(fs) / SR
    body = np.sin(2 * np.pi * 210 * t) * np.exp(-t / 0.12)            # 200 Hz body + 3rd partial so phones hear it
    return (np.sin(ph) + 0.5 * np.sin(3 * ph) + 0.35 * body) * env(n, 0.003, d)
def bell(f, d=1.6):
    n = int(4.8 * SR); t = np.arange(n) / SR
    s = sum(a * np.sin(2 * np.pi * f * m * t) * np.exp(-t / (d / m ** 0.5)) for m, a in [(1, 1), (2.01, .35), (3.02, .18), (4.2, .08)])
    return s * env(n, 0.004, 9)
def shimmer(dur=0.9):
    n = int(dur * SR); t = np.arange(n) / SR
    s = sum(np.sin(2 * np.pi * f * t + rng.uniform(0, 6)) for f in (1760, 2217, 2637, 3520))
    return s * np.sin(np.pi * t / dur) ** 2 * 0.25

# ---- cues ----
for e in EV:
    t, kind = e[0], e[1]
    if kind == 'type':
        add(click(rng.uniform(2600, 3600), rng.uniform(0.008, 0.014)), t + rng.uniform(-0.004, 0.004), rng.uniform(0.10, 0.15), rng.uniform(-0.3, 0.3))
    elif kind == 'return':
        add(click(1800, 0.02), t, 0.13)
    elif kind == 'whoosh':                      # code card shrinks, diagram card enters: cover the motion
        add(whoosh(0.8), t, 0.040)
    elif kind == 'node':
        k = e[2]; add(pop(520 * 2 ** (k / 6)), t, 0.16, pan=(-0.2 + 0.4 * (k % 2)))
    elif kind == 'punch':
        add(thump(), t, 0.18); add(shimmer(1.0), t + 0.02, 0.19)
    elif kind == 'thumb':
        i = e[2]; add(pop(880 * 2 ** ((i % 6) / 6), 0.05, 0.3), t, 0.08, pan=(-0.5 + (i % 4) / 3))
    elif kind == 'scene':
        add(whoosh(0.55), t - 0.02, 0.018)
    elif kind == 'chime':
        for f, dt in [(523.25, 0), (659.25, 0.06), (783.99, 0.12), (1046.5, 0.2)]:
            add(bell(f), t + dt, 0.05)
    elif kind == 'click':
        add(click(2200, 0.02), t, 0.22)
    elif kind == 'enter':
        add(click(1700, 0.022), t, 0.17)
    elif kind == 'key':
        add(click(rng.uniform(2600, 3400), 0.012), t, 0.14)
    elif kind == 'tick':
        add(click(rng.uniform(1300, 1700), 0.007), t, 0.07, pan=rng.uniform(-0.2, 0.2))
    elif kind == 'redraw':
        add(pop(760, 0.06, 0.3), t, 0.10)
# ---- pad: A-minor add9, voiced high enough for phone speakers (no low major second) ----
t = np.arange(N) / SR
pad = np.zeros(N)
# A minor add9 without B (no low major second, no semitone against the C5 chime); weight above 450 Hz for phones
PAD = [(110, .30), (164.81, .22), (220, .24), (329.63, .16), (440, .16), (659.26, .22), (880, .13), (1318.5, .05)]
for f, a in PAD:
    pad += a * np.sin(2 * np.pi * f * t + 0.3 * np.sin(2 * np.pi * 0.11 * t))
pad *= (0.75 + 0.25 * np.sin(2 * np.pi * 0.07 * t))
pad *= np.minimum(1, t / 1.5) * np.clip((DUR - t) / 1.2, 0, 1) * 0.030
if 'padonly' in sys.argv: L[:] = 0; R[:] = 0
L += pad; R += pad * 0.97
mix = np.stack([L, R], 1) * np.clip((DUR - t) / 0.4, 0, 1)[:, None]
GAIN = 1.6                                         # fixed master gain (no normalising to the loudest peak)
mix = np.tanh(mix * GAIN)
with wave.open('sfx.wav', 'wb') as w:
    w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR)
    w.writeframes((np.clip(mix, -1, 1) * 32767).astype('<i2').tobytes())
print('peak', float(np.max(np.abs(mix))), 'events', len(EV))
