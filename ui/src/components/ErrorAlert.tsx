import { AlertTriangle } from 'lucide-react';
import { ApiError } from '@/api/client';

export function ErrorAlert({ error, title }: { error: unknown; title?: string }) {
  const message =
    error instanceof ApiError
      ? error.message
      : error instanceof Error
        ? error.message
        : 'Something went wrong.';
  return (
    <div role="alert" className="flex items-start gap-3 rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
      <AlertTriangle aria-hidden className="mt-0.5 h-4 w-4 flex-shrink-0 text-red-500" />
      <div>
        {title ? <div className="font-medium">{title}</div> : null}
        <div>{message}</div>
      </div>
    </div>
  );
}
