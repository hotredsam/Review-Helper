import { useGrillStore } from "../store/grillStore";

export const DEFAULT_DEPTH = 3;

/** Depth slider: scales how many questions a grill round targets (~1–5h). */
export function DepthSlider({ projectId }: { projectId: number }) {
  const depth = useGrillStore((s) => s.depth[projectId] ?? DEFAULT_DEPTH);
  const setDepth = useGrillStore((s) => s.setDepth);
  const id = `depth-${projectId}`;
  return (
    <div className="flex items-center gap-2">
      <label htmlFor={id} className="text-xs text-fg-subtle">
        Depth
      </label>
      <input
        id={id}
        type="range"
        min={1}
        max={5}
        step={1}
        value={depth}
        onChange={(e) => setDepth(projectId, Number(e.target.value))}
        className="accent-accent"
        aria-label="Grill depth"
      />
      <span className="w-8 text-xs tabular-nums text-fg-subtle">~{depth}h</span>
    </div>
  );
}
