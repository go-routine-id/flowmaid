// Shot 3 `speed`: the reading as a pure function of the shot-local frame, in LOG units
// (log10 of "times mermaid.js"). The tape position is linear in log units (300 px / decade),
// so every easing below shapes what the eye sees on the tape.
//
// Card gauge-readout-moves · tape-scroll-fixed-pointer, adapted (DESIGN.md 8.3 / 10):
//   parked at the old limit → creep → limit breaks → anticipation → sprint (bezier .55,0,.25,1,
//   22 f, peak ≈ 83 px/f with speed blur) → spring brake with ONE visible swing → locked constant.
//   Overshoot: the sprint ends 8 % past the final value (122 vs 113) and the brake swing is 8 %
//   of that excess (card ≈ 17 %), reduced for the professional-trust motion tokens.
import { EASE, bezier, mix, ramp } from '../../lib/Polish';
import { SPEED_BEATS as B } from './beats';

export const PX_PER_DECADE = 300;
export const V_LOCK = 113;
const LG = Math.log10;
const L_PARK = 0; // 1×
const L_CREEP = LG(1.12);
const L_ANT = LG(0.95);
const L_PEAK = LG(122);
const L_LOCK = LG(V_LOCK);

const SPRINT = bezier(0.55, 0, 0.25, 1);

// Brake: under-damped spring released from rest at the peak, closed form (frame-pure, also for
// fractional frames, which velocity() samples). Swing ratio 8 % → damping ratio ζ ≈ 0.63.
const SWING = 0.08;
const ZETA = -Math.log(SWING) / Math.sqrt(Math.PI * Math.PI + Math.log(SWING) * Math.log(SWING));
const WN = 19.5; // rad/s: first (and only visible) swing bottoms out ~6 f after release
const WD = WN * Math.sqrt(1 - ZETA * ZETA);
const brakeResidual = (tSec: number) =>
  Math.exp(-ZETA * WN * tSec) * (Math.cos(WD * tSec) + ((ZETA * WN) / WD) * Math.sin(WD * tSec));

/** log10 of the reading at shot-local frame f. */
export const logValueAt = (f: number): number => {
  if (f <= B.creep) return L_PARK;
  if (f <= B.breakAt) return mix(L_PARK, L_CREEP, ramp(f, B.creep, B.breakAt - B.creep, EASE.swift));
  if (f <= B.antAt) return L_CREEP;
  if (f <= B.sprintAt) return mix(L_CREEP, L_ANT, ramp(f, B.antAt, B.sprintAt - B.antAt, EASE.smooth));
  if (f <= B.brakeAt) return mix(L_ANT, L_PEAK, SPRINT((f - B.sprintAt) / B.sprintDur));
  if (f >= B.lockAt) return L_LOCK; // locked constant: no tail drift
  return L_LOCK + (L_PEAK - L_LOCK) * brakeResidual((f - B.brakeAt) / 30);
};

export const valueAt = (f: number) => Math.pow(10, logValueAt(f));
