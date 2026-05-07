import { Children, cloneElement, forwardRef, isValidElement, useId } from 'react';
import type {
  InputHTMLAttributes,
  ReactElement,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from 'react';
import { cn } from '@/lib/cn';

const baseField =
  'block w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm text-slate-900 shadow-sm placeholder:text-slate-400 focus:border-brand-500 focus:ring-1 focus:ring-brand-500 focus:outline-none disabled:bg-slate-50 disabled:text-slate-500 aria-[invalid=true]:border-red-500 aria-[invalid=true]:focus:border-red-500 aria-[invalid=true]:focus:ring-red-500';

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input ref={ref} className={cn(baseField, className)} {...props} />
  ),
);
Input.displayName = 'Input';

export const Textarea = forwardRef<
  HTMLTextAreaElement,
  TextareaHTMLAttributes<HTMLTextAreaElement>
>(({ className, ...props }, ref) => (
  <textarea ref={ref} className={cn(baseField, 'min-h-[80px] resize-y font-mono', className)} {...props} />
));
Textarea.displayName = 'Textarea';

export const Select = forwardRef<
  HTMLSelectElement,
  SelectHTMLAttributes<HTMLSelectElement>
>(({ className, children, ...props }, ref) => (
  <select ref={ref} className={cn(baseField, 'pr-8', className)} {...props}>
    {children}
  </select>
));
Select.displayName = 'Select';

interface FieldProps {
  label: string;
  htmlFor?: string;
  hint?: string;
  error?: string;
  required?: boolean;
  readOnly?: boolean;
  children: React.ReactNode;
  className?: string;
}

type FieldChildProps = {
  'aria-invalid'?: boolean | 'true' | 'false';
  'aria-describedby'?: string;
  id?: string;
};

export function Field({
  label,
  htmlFor,
  hint,
  error,
  required,
  readOnly,
  children,
  className,
}: FieldProps) {
  const reactId = useId();
  const errorId = `${reactId}-error`;
  const hintId = `${reactId}-hint`;

  // Inject aria-invalid + aria-describedby into the field child so callers
  // don't have to wire it up manually. Preserves any explicit values the
  // caller already passed.
  const child =
    Children.count(children) === 1 && isValidElement(children)
      ? cloneElement(children as ReactElement<FieldChildProps>, {
          'aria-invalid':
            (children.props as FieldChildProps)['aria-invalid'] ??
            (error ? true : undefined),
          'aria-describedby':
            (children.props as FieldChildProps)['aria-describedby'] ??
            (error ? errorId : hint ? hintId : undefined),
        })
      : children;

  return (
    <div className={cn('flex flex-col gap-1', className)}>
      <label htmlFor={htmlFor} className="flex items-center gap-1.5 text-sm font-medium text-slate-700">
        <span>
          {label}
          {required ? <span className="ml-0.5 text-red-500">*</span> : null}
        </span>
        {readOnly ? (
          <span className="rounded bg-ink-100 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-ink-500">
            Read-only
          </span>
        ) : null}
      </label>
      {child}
      {error ? (
        <p id={errorId} className="text-xs text-red-600">
          {error}
        </p>
      ) : hint ? (
        <p id={hintId} className="text-xs text-slate-500">
          {hint}
        </p>
      ) : null}
    </div>
  );
}
