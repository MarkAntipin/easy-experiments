import type { ExperimentStatus } from '@/api/types';
import { cn } from '@/lib/cn';

const STATUS_STYLES: Record<ExperimentStatus, string> = {
  draft: 'bg-ink-100 text-ink-700 ring-ink-200',
  running: 'bg-emerald-50 text-emerald-700 ring-emerald-200',
  stopped: 'bg-amber-50 text-amber-800 ring-amber-200',
  deleted: 'bg-red-50 text-red-700 ring-red-200',
};

export function StatusBadge({ status }: { status: ExperimentStatus }) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full px-2.5 py-0.5 text-sm font-medium ring-1 ring-inset capitalize',
        STATUS_STYLES[status],
      )}
    >
      {status}
    </span>
  );
}
