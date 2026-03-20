// Presentation Layout — DashboardLayout
// Collapsible sidebar + topbar with dark-mode toggle.
// Renders child routes via <Outlet />.

import { useState } from 'react';
import { Outlet, NavLink, useNavigate } from 'react-router-dom';
import {
  LayoutDashboard,
  ShieldCheck,
  Activity,
  Database,
  Settings as SettingsIcon,
  ChevronLeft,
  ChevronRight,
  Sun,
  Moon,
  LogOut,
  Key,
} from 'lucide-react';
import { useAuth } from '../context/AuthContext';
import { useTheme } from '../hooks/useTheme';

const NAV_ITEMS = [
  { to: '/dashboard',             label: 'Dashboard',          icon: LayoutDashboard, end: true },
  { to: '/dashboard/api-keys',    label: 'API Keys',           icon: Key,             end: false },
  { to: '/dashboard/compliance',  label: 'Compliance',         icon: ShieldCheck,     end: false },
  { to: '/dashboard/traces',      label: 'Trace Explorer',     icon: Activity,        end: false },
  { to: '/dashboard/cache',       label: 'Semantic Cache',     icon: Database,        end: false },
  { to: '/dashboard/settings',    label: 'Settings',           icon: SettingsIcon,    end: false },
];

export function DashboardLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const { user, logout } = useAuth();
  const { theme, toggleTheme } = useTheme();
  const navigate = useNavigate();

  function handleLogout() {
    logout();
    navigate('/login');
  }

  return (
    <div className="min-h-screen bg-slate-950 dark:bg-slate-950 text-slate-100 flex">
      {/* Sidebar */}
      <aside
        className={`hidden md:flex flex-col border-r border-slate-800/80 bg-slate-900/80 backdrop-blur-xl transition-[width] duration-300 ease-in-out flex-shrink-0 ${
          collapsed ? 'w-16' : 'w-64 lg:w-72'
        }`}
      >
        {/* Logo row */}
        <div className={`h-14 border-b border-slate-800/80 flex items-center ${collapsed ? 'justify-center px-0' : 'justify-between px-4'}`}>
          {!collapsed && (
            <div className="flex items-center gap-2">
              <div className="h-7 w-7 rounded-lg bg-indigo-600 flex items-center justify-center shadow-[0_0_16px_rgba(99,102,241,0.5)]">
                <span className="text-[10px] font-bold tracking-tight text-white">RE</span>
              </div>
              <div>
                <p className="text-[10px] uppercase tracking-[0.2em] text-slate-500">RedEye</p>
                <p className="text-xs font-semibold text-slate-50 leading-none">Control Plane</p>
              </div>
            </div>
          )}
          <button
            onClick={() => setCollapsed((c) => !c)}
            className="p-1.5 rounded-md text-slate-500 hover:text-slate-200 hover:bg-slate-800 transition-colors"
            aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {collapsed ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />}
          </button>
        </div>

        {/* Nav */}
        <nav className="flex-1 px-2 py-3 space-y-0.5 overflow-y-auto custom-scrollbar">
          {NAV_ITEMS.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                `w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors border ${
                  isActive
                    ? 'bg-indigo-600/15 text-indigo-300 border-indigo-500/30 shadow-[0_0_16px_rgba(99,102,241,0.15)]'
                    : 'bg-transparent text-slate-400 border-transparent hover:bg-slate-800/50 hover:text-slate-100'
                } ${collapsed ? 'justify-center' : ''}`
              }
              title={collapsed ? label : undefined}
            >
              <Icon className="w-4 h-4 flex-shrink-0" />
              {!collapsed && <span>{label}</span>}
            </NavLink>
          ))}
        </nav>

        {/* Footer */}
        {!collapsed && (
          <div className="px-4 pb-4 pt-2 border-t border-slate-800/80 text-[11px] text-slate-500">
            <p className="flex items-center justify-between">
              <span>Cluster</span>
              <span className="font-mono text-indigo-400">local-dev</span>
            </p>
            <p className="mt-1 text-slate-600">@8080 · @8081 · @8082 · @8083</p>
          </div>
        )}
      </aside>

      {/* Main area */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Topbar */}
        <header className="h-14 border-b border-slate-800/80 bg-slate-900/70 backdrop-blur-xl px-4 sm:px-6 flex items-center justify-between flex-shrink-0">
          {/* Mobile brand */}
          <div className="flex items-center gap-2 md:hidden">
            <div className="h-7 w-7 rounded-lg bg-indigo-600 flex items-center justify-center">
              <span className="text-[10px] font-bold text-white">RE</span>
            </div>
            <span className="text-xs font-semibold text-slate-100">RedEye</span>
          </div>

          <div className="hidden md:block" />

          {/* Right controls */}
          <div className="flex items-center gap-2">
            {/* Dark mode toggle */}
            <button
              onClick={toggleTheme}
              className="p-2 rounded-lg text-slate-400 hover:text-slate-100 hover:bg-slate-800 transition-colors"
              aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
            >
              {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>

            {/* User chip */}
            {user && (
              <div className="hidden sm:flex items-center gap-2 pl-2 border-l border-slate-800 ml-1">
                <div className="h-7 w-7 rounded-full bg-indigo-600/30 border border-indigo-500/30 flex items-center justify-center text-[11px] font-bold text-indigo-300">
                  {user.email[0].toUpperCase()}
                </div>
                <span className="text-xs text-slate-400 max-w-[120px] truncate">{user.email}</span>
              </div>
            )}

            {/* Logout */}
            <button
              onClick={handleLogout}
              className="p-2 rounded-lg text-slate-500 hover:text-rose-400 hover:bg-slate-800 transition-colors"
              aria-label="Sign out"
            >
              <LogOut className="w-4 h-4" />
            </button>
          </div>
        </header>

        {/* Mobile nav (horizontal scroll) */}
        <nav className="md:hidden overflow-x-auto flex gap-1 px-3 py-2 border-b border-slate-800/60 bg-slate-900/60 custom-scrollbar">
          {NAV_ITEMS.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                `flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium whitespace-nowrap transition-colors ${
                  isActive
                    ? 'bg-indigo-600/20 text-indigo-300 border border-indigo-500/30'
                    : 'text-slate-400 border border-transparent hover:text-slate-100'
                }`
              }
            >
              <Icon className="w-3.5 h-3.5" />
              {label}
            </NavLink>
          ))}
        </nav>

        {/* Content */}
        <main className="flex-1 overflow-y-auto custom-scrollbar">
          <div className="max-w-7xl mx-auto w-full px-4 sm:px-6 lg:px-8 py-6 lg:py-8">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
