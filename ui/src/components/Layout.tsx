import { NavLink, Outlet } from 'react-router-dom';
import { Beaker, KeyRound, LogOut } from 'lucide-react';
import { useAuth } from '@/auth/AuthContext';
import { LogoMark } from '@/components/Logo';
import { cn } from '@/lib/cn';

function NavItem({
  to,
  icon: Icon,
  label,
}: {
  to: string;
  icon: typeof Beaker;
  label: string;
}) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(
          'group relative flex items-center gap-3 rounded-md px-3 py-2.5 text-base font-medium transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500',
          isActive
            ? 'bg-brand-50 text-brand-700'
            : 'text-ink-500 hover:bg-slate-100 hover:text-ink-900',
        )
      }
    >
      {({ isActive }) => (
        <>
          <span
            aria-hidden
            className={cn(
              'absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-r bg-brand-gradient transition-opacity',
              isActive ? 'opacity-100' : 'opacity-0',
            )}
          />
          <Icon
            aria-hidden
            className={cn('h-5 w-5', isActive ? 'text-brand-600' : 'text-ink-400 group-hover:text-ink-700')}
          />
          {label}
        </>
      )}
    </NavLink>
  );
}

export function Layout() {
  const { session, logout } = useAuth();

  return (
    <div className="flex min-h-screen bg-slate-50">
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-3 focus:top-3 focus:z-50 focus:rounded-md focus:bg-white focus:px-3 focus:py-2 focus:text-sm focus:font-medium focus:text-brand-700 focus:shadow-md focus:ring-2 focus:ring-brand-500"
      >
        Skip to main content
      </a>
      <aside className="flex w-64 shrink-0 flex-col border-r border-slate-200 bg-white">
        <div className="flex h-16 items-center gap-3 border-b border-slate-200 px-5">
          <LogoMark size={32} />
          <span className="text-base font-semibold leading-none tracking-tight">
            <span className="text-brand-600">Easy</span>
            <span className="text-ink-900">Experiments</span>
          </span>
        </div>
        <nav aria-label="Main" className="flex-1 space-y-1 p-3">
          <NavItem to="/experiments" icon={Beaker} label="Experiments" />
          <NavItem to="/api-keys" icon={KeyRound} label="API Keys" />
        </nav>
        <div className="border-t border-slate-200 p-3">
          {session ? (
            <div className="mb-2 flex items-center gap-2.5 rounded-md px-2 py-2">
              {session.user.pictureUrl ? (
                <img
                  src={session.user.pictureUrl}
                  alt=""
                  className="h-9 w-9 rounded-full ring-2 ring-brand-100"
                  referrerPolicy="no-referrer"
                />
              ) : (
                <div
                  aria-hidden
                  className="grid h-9 w-9 place-items-center rounded-full bg-brand-gradient text-sm font-semibold text-white"
                >
                  {session.user.email.slice(0, 1).toUpperCase()}
                </div>
              )}
              <div className="min-w-0 flex-1 leading-tight">
                <div className="truncate text-base font-medium text-ink-900">
                  {session.user.name ?? session.user.email}
                </div>
                <div className="truncate text-sm text-ink-500">
                  {session.company.name}
                </div>
              </div>
            </div>
          ) : null}
          <button
            type="button"
            onClick={logout}
            className="flex w-full items-center gap-2.5 rounded-md px-3 py-2.5 text-base font-medium text-ink-500 transition hover:bg-slate-100 hover:text-ink-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500"
          >
            <LogOut aria-hidden className="h-5 w-5" />
            Sign out
          </button>
        </div>
      </aside>
      <main id="main-content" className="flex-1 overflow-x-hidden">
        <Outlet />
      </main>
    </div>
  );
}
