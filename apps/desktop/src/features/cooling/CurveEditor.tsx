import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import type { FanCurve } from "@/lib/api/control";
import { movePoint } from "./curve";

// Plot area in SVG units; the SVG scales to its container width.
const W = 560;
const H = 260;
const PAD = { left: 40, right: 16, top: 16, bottom: 32 };
const T_MIN = 0;
const T_MAX = 110;

const x = (t: number) => PAD.left + ((t - T_MIN) / (T_MAX - T_MIN)) * (W - PAD.left - PAD.right);
const y = (d: number) => H - PAD.bottom - (d / 100) * (H - PAD.top - PAD.bottom);
const tAt = (px: number) => T_MIN + ((px - PAD.left) / (W - PAD.left - PAD.right)) * (T_MAX - T_MIN);
const dAt = (py: number) => ((H - PAD.bottom - py) / (H - PAD.top - PAD.bottom)) * 100;

/**
 * Draggable fan curve. Points stay valid while dragging (see `movePoint`).
 * Focused points accept arrow keys (Shift = 5 steps). `preview` draws the
 * daemon's simulated duty for a temperature.
 */
export function CurveEditor({
  curve,
  selected,
  preview,
  onSelect,
  onChange,
}: {
  curve: FanCurve;
  selected: number | null;
  preview?: { temperature_c: number; duty_percent: number | null };
  onSelect: (index: number) => void;
  onChange: (curve: FanCurve) => void;
}) {
  const svg = useRef<SVGSVGElement>(null);
  const [dragging, setDragging] = useState<number | null>(null);

  const toSvg = (e: PointerEvent) => {
    const r = svg.current!.getBoundingClientRect();
    return [((e.clientX - r.left) / r.width) * W, ((e.clientY - r.top) / r.height) * H] as const;
  };
  const drag = (e: PointerEvent) => {
    if (dragging === null) return;
    const [px, py] = toSvg(e);
    onChange(movePoint(curve, dragging, Math.round(tAt(px)), Math.round(dAt(py))));
  };
  const key = (e: KeyboardEvent, i: number) => {
    const step = e.shiftKey ? 5 : 1;
    const d = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] }[e.key];
    if (!d) return;
    e.preventDefault();
    const p = curve.points[i]!;
    onChange(movePoint(curve, i, p.temperature_c + d[0]!, p.duty_percent + d[1]!));
  };

  const line = curve.points.map((p) => `${x(p.temperature_c)},${y(p.duty_percent)}`).join(" ");
  const first = curve.points[0]!;
  const last = curve.points[curve.points.length - 1]!;
  // Below the first point the daemon holds its duty; past critical it runs 100 %.
  const full = `${x(T_MIN)},${y(first.duty_percent)} ${line} ${x(curve.critical_c)},${y(last.duty_percent)} ${x(curve.critical_c)},${y(100)} ${x(T_MAX)},${y(100)}`;

  return (
    <svg
      ref={svg}
      viewBox={`0 0 ${W} ${H}`}
      className="w-full touch-none select-none"
      role="group"
      aria-label="Fan curve: duty over temperature"
      onPointerMove={drag}
      onPointerUp={() => setDragging(null)}
      onPointerLeave={() => setDragging(null)}
    >
      {[0, 25, 50, 75, 100].map((d) => (
        <g key={d}>
          <line x1={PAD.left} x2={W - PAD.right} y1={y(d)} y2={y(d)} stroke="rgb(255 255 255 / 0.06)" />
          <text x={PAD.left - 8} y={y(d) + 4} textAnchor="end" className="fill-text-subtle text-[11px] tabular">
            {d}%
          </text>
        </g>
      ))}
      {[0, 20, 40, 60, 80, 100].map((t) => (
        <text key={t} x={x(t)} y={H - 10} textAnchor="middle" className="fill-text-subtle text-[11px] tabular">
          {t}°
        </text>
      ))}
      <line x1={PAD.left} x2={W - PAD.right} y1={y(curve.minimum_percent)} y2={y(curve.minimum_percent)} stroke="#8f8f8f" strokeDasharray="3 4" />
      <line x1={x(curve.critical_c)} x2={x(curve.critical_c)} y1={PAD.top} y2={H - PAD.bottom} stroke="#ef4444" strokeDasharray="3 4" />
      <text x={x(curve.critical_c) + 4} y={PAD.top + 10} className="fill-danger-text text-[11px]">
        critical
      </text>
      <polyline points={full} fill="none" stroke="#10a37f" strokeWidth={2} strokeLinejoin="round" />
      {preview && (
        <g aria-hidden>
          <line x1={x(preview.temperature_c)} x2={x(preview.temperature_c)} y1={PAD.top} y2={H - PAD.bottom} stroke="#60a5fa" strokeOpacity={0.6} />
          {preview.duty_percent !== null && <circle cx={x(preview.temperature_c)} cy={y(preview.duty_percent)} r={4} fill="#60a5fa" />}
        </g>
      )}
      {curve.points.map((p, i) => (
        <circle
          key={i}
          cx={x(p.temperature_c)}
          cy={y(p.duty_percent)}
          r={selected === i ? 7 : 6}
          tabIndex={0}
          role="slider"
          aria-label={`Point ${i + 1}`}
          aria-valuetext={`${p.temperature_c} °C, ${p.duty_percent} %`}
          className="cursor-grab outline-none focus-visible:stroke-info"
          fill={selected === i ? "#ececec" : "#10a37f"}
          stroke="#000"
          strokeWidth={2}
          onPointerDown={(e) => {
            (e.currentTarget.ownerSVGElement as SVGSVGElement).setPointerCapture(e.pointerId);
            setDragging(i);
            onSelect(i);
          }}
          onFocus={() => onSelect(i)}
          onKeyDown={(e) => key(e, i)}
        />
      ))}
    </svg>
  );
}
