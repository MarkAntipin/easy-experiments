import { cn } from '@/lib/cn';

interface LogoProps {
  className?: string;
  /** Visual size (height in px). Sources the closest pre-rendered PNG. */
  size?: number;
}

/** Flask-only icon (logomark). Use in compact spots like the sidebar header. */
export function LogoMark({ className, size = 32 }: LogoProps) {
  const src = size <= 64 ? '/logo-mark-64.png' : size <= 128 ? '/logo-mark-128.png' : '/logo-mark-256.png';
  return (
    <img
      src={src}
      alt=""
      width={size}
      height={size}
      className={cn('select-none', className)}
      draggable={false}
    />
  );
}

/** Full lockup with wordmark — flask + "EasyExperiments" text. */
export function Logo({ className, size = 128 }: LogoProps) {
  const src = size <= 128 ? '/logo-128.png' : size <= 256 ? '/logo-256.png' : '/logo-512.png';
  return (
    <img
      src={src}
      alt="Easy Experiments"
      width={size}
      height={size}
      className={cn('select-none', className)}
      draggable={false}
    />
  );
}
