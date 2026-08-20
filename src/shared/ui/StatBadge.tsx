export interface DiffStat { add: number; del: number }

export function StatBadge({ stat }: { stat: DiffStat }) {
  return (
    <span className="file-tree-stats">
      {stat.add > 0 && <span className="diff-add">+{stat.add}</span>}
      {stat.del > 0 && <span className="diff-del">-{stat.del}</span>}
    </span>
  );
}
