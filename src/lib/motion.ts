import { cubicOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";

/** Whether the user asked the system to reduce motion. */
export function reducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

type Phase = { direction?: "in" | "out" | "both" };

/**
 * Moves content along the time axis: `direction` 1 for later (next month), -1 for
 * earlier, 0 to just fade. Later lies to the left in right-to-left layouts, so new
 * content comes from the left and old content leaves to the right there, and the
 * other way round in left-to-right ones.
 */
export function timeline(
  node: Element,
  { direction = 0, distance = 28, duration = 280 }: { direction?: number; distance?: number; duration?: number },
  phase?: Phase,
): TransitionConfig {
  const leaving = phase?.direction === "out";
  const later = getComputedStyle(node).direction === "rtl" ? -1 : 1;
  // Where entering content starts, or leaving content ends up.
  const offset = (leaving ? -1 : 1) * later * direction * distance;
  return {
    duration: reducedMotion() ? 0 : leaving ? duration * 0.75 : duration,
    easing: cubicOut,
    css: (t) => `transform: translateX(${(1 - t) * offset}px); opacity: ${t};`,
  };
}

/** A short rise and fade, for content that changes in place. */
export function rise(node: Element, { y = 5, duration = 180 }: { y?: number; duration?: number } = {}): TransitionConfig {
  return {
    duration: reducedMotion() ? 0 : duration,
    easing: cubicOut,
    css: (t) => `transform: translateY(${(1 - t) * y}px); opacity: ${t};`,
  };
}

/** Grows in from a little smaller, for small controls that appear and disappear. */
export function pop(node: Element, { duration = 160 }: { duration?: number } = {}): TransitionConfig {
  return {
    duration: reducedMotion() ? 0 : duration,
    easing: cubicOut,
    css: (t) => `transform: scale(${0.85 + 0.15 * t}); opacity: ${t};`,
  };
}

/** -1, 0 or 1: how month `b` lies relative to month `a`. */
export function monthDirection(a: { year: number; month: number } | null, b: { year: number; month: number }) {
  if (!a) return 0;
  return Math.sign(b.year * 12 + b.month - (a.year * 12 + a.month));
}

/**
 * Moves between levels of the calendar (days → months → years): `level` 1 going up,
 * -1 coming back down, 0 to just fade. Going up, the level left behind shrinks away
 * and the new one settles from slightly larger, like zooming out; down is the reverse.
 */
export function zoom(
  node: Element,
  { level = 0, duration = 260 }: { level?: number; duration?: number },
  phase?: Phase,
): TransitionConfig {
  const leaving = phase?.direction === "out";
  const scale = level === 0 ? 1 : leaving === level > 0 ? 0.92 : 1.06;
  return {
    duration: reducedMotion() ? 0 : leaving ? duration * 0.7 : duration,
    easing: cubicOut,
    css: (t) => `transform: scale(${scale + (1 - scale) * t}); opacity: ${t};`,
  };
}
