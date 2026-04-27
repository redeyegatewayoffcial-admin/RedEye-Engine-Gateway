// Presentation Layout — DashboardLayout
// Collapsible sidebar + topbar with dark-mode toggle.
// Theme: "The Obsidian Command" — Physical Lab Equipment aesthetic.

import { useState, useEffect } from 'react';
import { Outlet, NavLink, useNavigate, Link } from 'react-router-dom';
import {
  LayoutDashboard, ShieldCheck, Siren, Activity, Database,
  Settings as SettingsIcon, ChevronLeft, ChevronRight,
  Sun, Moon, LogOut, Key, CreditCard, Search, Command, User,
  Terminal, LifeBuoy
} from 'lucide-react';
import { motion, AnimatePresence, LayoutGroup } from 'framer-motion';

import { useAuth } from '../context/AuthContext';
import { useTheme } from '../hooks/useTheme';
import { useIncident } from '../context/IncidentContext';
import { OmniPalette } from '../components/ui/OmniPalette';

const NAV_ITEMS = [
  { to: '/dashboard',            label: 'Dashboard',      icon: LayoutDashboard, end: true  },
  { to: '/dashboard/api-keys',   label: 'API Keys',       icon: Key,             end: false },
  { to: '/dashboard/billing',    label: 'Cost & Billing', icon: CreditCard,      end: false },
  { to: '/dashboard/compliance', label: 'Compliance',     icon: ShieldCheck,     end: false },
  { to: '/dashboard/security',   label: 'Security',       icon: Siren,           end: false },
  { to: '/dashboard/traces',     label: 'Trace Explorer', icon: Activity,        end: false },
  { to: '/dashboard/cache',      label: 'Semantic Cache', icon: Database,        end: false },
  { to: '/dashboard/settings',   label: 'Settings',       icon: SettingsIcon,    end: false },
  { to: '/dashboard/profile',    label: 'Profile',        icon: User,            end: false },
];

/**
 * Spatial Tooltip for Collapsed State
 */
function SidebarTooltip({ label, collapsed, children }: { label: string; collapsed: boolean; children: React.ReactNode }) {
  const [isHovered, setIsHovered] = useState(false);

  if (!collapsed) return <>{children}</>;

  return (
    <div 
      className="relative flex items-center justify-center"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {children}
      <AnimatePresence>
        {isHovered && (
          <motion.div
            initial={{ opacity: 0, x: 10, scale: 0.95 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: 10, scale: 0.95 }}
            transition={{ duration: 0.15, ease: "easeOut" }}
            className="absolute left-full ml-4 z-[100] pointer-events-none"
          >
            <div 
              className="px-3 py-1.5 rounded-lg border border-white/10 shadow-2xl backdrop-blur-[24px] saturate-[200%] bg-[rgba(20,20,20,0.8)] flex items-center justify-center"
            >
              <span className="font-geist text-xs font-medium text-white/90 whitespace-nowrap tracking-wide">
                {label}
              </span>
            </div>
            {/* Liquid Glass Arrow Pointer */}
            <div className="absolute left-[-4px] top-1/2 -translate-y-1/2 w-2 h-2 rotate-45 bg-[rgba(20,20,20,0.8)] border-l border-b border-white/10" />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

/**
 * Magnetic Sidebar Nav Item
 */
function SidebarNavItem({ to, label, icon: Icon, end, collapsed }: any) {
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) =>
        `relative group flex items-center gap-3 px-4 py-3 rounded-xl text-sm font-medium transition-all duration-500 font-geist ${
          collapsed ? 'justify-center px-0' : ''
        } ${
          isActive 
            ? 'text-[var(--on-surface)] drop-shadow-[0_0_8px_rgba(34,211,238,0.4)]' 
            : 'text-[var(--on-surface-muted)] hover:text-[var(--on-surface)]'
        }`
      }
    >
      {({ isActive }) => (
        <>
          {isActive && (
            <motion.div
              layoutId="sidebar-active-indicator"
              className="w-[2px] h-full absolute left-0 bg-[var(--accent-cyan)] shadow-[0_0_10px_var(--accent-cyan)] z-20"
              transition={{ type: "spring", stiffness: 350, damping: 30 }}
            />
          )}
          
          <SidebarTooltip label={label} collapsed={collapsed}>
            <div className="relative flex items-center gap-3">
              <motion.div
                whileHover={{ scale: 1.1, rotate: 5 }}
                whileTap={{ scale: 0.95 }}
                className={`relative z-10 transition-colors duration-300 ${isActive ? 'text-[var(--accent-cyan)]' : 'group-hover:text-[var(--on-surface)]'}`}
              >
                <Icon className="w-4 h-4 flex-shrink-0" />
              </motion.div>
              
              {!collapsed && (
                <motion.span 
                  initial={false}
                  animate={{ opacity: 1, x: 0 }}
                  className="tracking-tight relative z-10"
                >
                  {label}
                </motion.span>
              )}
            </div>
          </SidebarTooltip>
        </>
      )}
    </NavLink>
  );
}

export function DashboardLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const [isOmniOpen, setIsOmniOpen] = useState(false);
  const [contextMode, setContextMode] = useState<'individual' | 'team'>('individual');
  const { isIncidentActive } = useIncident();
  const { user, logout } = useAuth();
  const { theme, toggleTheme } = useTheme();
  const navigate = useNavigate();

  useEffect(() => {
    if (isIncidentActive) setCollapsed(true);
  }, [isIncidentActive]);

  function handleLogout() {
    logout();
    navigate('/login');
  }

  return (
    <motion.div
      initial={false}
      animate={{
        padding: isIncidentActive ? '8px' : '0px',
        backgroundColor: 'var(--bg-canvas)',
      }}
      className="grain min-h-screen flex font-geist selection:bg-cyan-500/30 overflow-hidden"
      style={{
        color: 'var(--on-surface)',
        transition: 'color 0.3s ease',
      }}
    >
      {/* ── Ambient Mesh Background (Dark Mode Only) ──────────────── */}
      <div className="fixed inset-0 overflow-hidden pointer-events-none z-0 hidden dark:block">
        <motion.div 
          animate={{
            rotate: isIncidentActive ? 360 : 0,
            scale: isIncidentActive ? 1.5 : 1,
          }}
          transition={{
            rotate: { duration: isIncidentActive ? 10 : 60, repeat: Infinity, ease: "linear" },
            scale: { duration: 2 }
          }}
          className={`absolute -top-[10%] -left-[10%] h-[40%] bg-amber-500/15 blur-[120px] rounded-full mix-blend-screen transition-all duration-1000 ${isIncidentActive ? 'w-[60%] animate-incident-pulse' : 'w-[40%]'}`} 
        />
        <motion.div 
          animate={{
            rotate: isIncidentActive ? -360 : 0,
            scale: isIncidentActive ? 1.8 : 1,
          }}
          transition={{
            rotate: { duration: isIncidentActive ? 8 : 45, repeat: Infinity, ease: "linear" },
            scale: { duration: 2 }
          }}
          className={`absolute -top-[10%] -right-[10%] h-[40%] bg-rose-500/15 blur-[120px] rounded-full mix-blend-screen transition-all duration-1000 ${isIncidentActive ? 'w-[60%] animate-incident-pulse' : 'w-[35%]'}`} 
        />
        {!isIncidentActive && (
          <>
            <div className="absolute -bottom-[10%] -left-[10%] w-[40%] h-[40%] bg-cyan-500/15 blur-[120px] rounded-full mix-blend-screen" />
            <div className="absolute -bottom-[10%] -right-[10%] w-[30%] h-[30%] bg-yellow-500/10 blur-[100px] rounded-full mix-blend-screen" />
          </>
        )}
      </div>

      {/* ── System Stress Overlay ────────────────────────────────── */}
      <AnimatePresence>
        {isIncidentActive && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 pointer-events-none z-[100] shadow-[inset_0_0_100px_rgba(225,29,72,0.15)] ring-[12px] ring-rose-500/10"
          />
        )}
      </AnimatePresence>

      {/* ── Sidebar ─────────────────────────────────────────────── */}
      <aside
        id="tour-sidebar"
        className={`relative hidden md:flex flex-col flex-shrink-0 z-20 transition-all duration-500 cubic-bezier(0.4, 0, 0.2, 1) ${
          collapsed ? 'w-16' : 'w-64 lg:w-72'
        } ${isIncidentActive ? 'bg-rose-950/20 border-r border-rose-500/20' : 'bg-[var(--surface-lowest)]'}`}
        style={{
          boxShadow: 'inset -1px 0 0 0 rgba(255,255,255,0.04)',
          backdropFilter: 'blur(24px)',
          WebkitBackdropFilter: 'blur(24px)',
        }}
      >
        {/* Logo row */}
        <div
          className={`h-16 flex items-center flex-shrink-0 ${collapsed ? 'justify-center px-0' : 'justify-between px-5'}`}
          style={{ boxShadow: 'inset 0 -1px 0 0 rgba(255,255,255,0.04)' }}
        >
          {!collapsed && (
            <motion.div 
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              className="flex items-center gap-3"
            >
              <div className={`h-8 w-8 rounded-xl flex items-center justify-center transition-colors duration-500 ${isIncidentActive ? 'bg-rose-500 shadow-[0_0_20px_rgba(244,63,94,0.5)]' : 'bg-cyan-500 shadow-[0_0_20px_rgba(34,211,238,0.5)]'}`}>
                <span className="text-[11px] font-black tracking-tight text-[#050505]">RE</span>
              </div>
              <div>
                <p className="text-[9px] uppercase tracking-[0.25em] font-bold font-geist text-[var(--text-muted)]">RedEye</p>
                <p className="text-xs font-bold tracking-tight leading-none font-geist text-[var(--on-surface)]">Command</p>
              </div>
            </motion.div>
          )}
          <button
            onClick={() => setCollapsed(c => !c)}
            className="p-1.5 rounded-lg transition-colors text-[var(--text-muted)] hover:text-[var(--on-surface)] hover:bg-[var(--surface-container)]"
            aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {collapsed ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />}
          </button>
        </div>

        {/* Navigation List */}
        <nav className="flex-1 px-2 py-6 space-y-1 overflow-y-auto custom-scrollbar">
          <LayoutGroup>
            {NAV_ITEMS.map((item) => (
              <SidebarNavItem key={item.to} {...item} collapsed={collapsed} />
            ))}
          </LayoutGroup>
        </nav>

        {/* 🛠️ Utility Hub (Bottom Section) */}
        <div className={`mt-auto px-2 pb-6 flex flex-col gap-1 transition-all duration-300 ${collapsed ? 'items-center' : ''}`}>
          
          {/* Developer Protocol */}
          <SidebarNavItem 
            to="/docs" 
            label="Developer Protocol" 
            icon={Terminal} 
            collapsed={collapsed} 
          />

          {/* Command Support */}
          <SidebarNavItem 
            to="/support" 
            label="Command Support" 
            icon={LifeBuoy} 
            collapsed={collapsed} 
          />

          {/* Context Switcher Section */}
          <div className={`mt-4 px-2 ${collapsed ? 'w-full flex justify-center' : ''}`}>
            <div className={`bg-[var(--surface-bright)] p-1 rounded-xl flex gap-1 ${collapsed ? 'flex-col p-1' : ''}`}>
              <button 
                onClick={() => setContextMode('individual')}
                className={`flex-1 px-3 py-1.5 rounded-lg text-[9px] font-bold uppercase tracking-wider transition-all duration-300 ${
                  contextMode === 'individual' 
                    ? `bg-[var(--surface-lowest)] ${isIncidentActive ? 'text-rose-400' : 'text-[var(--accent-cyan)] shadow-sm shadow-cyan-500/10'}` 
                    : 'text-[var(--on-surface-muted)] hover:text-[var(--on-surface)]'
                } ${collapsed ? 'px-0 h-8 flex items-center justify-center' : ''}`}
              >
                {collapsed ? <User className="w-3.5 h-3.5" /> : 'Individual'}
              </button>
              <button 
                onClick={() => setContextMode('team')}
                className={`flex-1 px-3 py-1.5 rounded-lg text-[9px] font-bold uppercase tracking-wider transition-all duration-300 ${
                  contextMode === 'team' 
                    ? `bg-[var(--surface-lowest)] ${isIncidentActive ? 'text-rose-400' : 'text-[var(--accent-cyan)] shadow-sm shadow-cyan-500/10'}` 
                    : 'text-[var(--on-surface-muted)] hover:text-[var(--on-surface)]'
                } ${collapsed ? 'px-0 h-8 flex items-center justify-center' : ''}`}
              >
                {collapsed ? <ShieldCheck className="w-3.5 h-3.5" /> : 'Team / Org'}
              </button>
            </div>
          </div>
        </div>
      </aside>

      {/* ── Main area ───────────────────────────────────────────── */}
      <motion.div 
        animate={{
          gap: isIncidentActive ? '8px' : '0px',
        }}
        className="flex-1 flex flex-col min-w-0 h-screen overflow-hidden"
      >
        {/* Topbar */}
        <header
          className="h-16 px-4 sm:px-6 flex items-center justify-between flex-shrink-0 z-10 transition-colors duration-500"
          style={{
            backgroundColor: isIncidentActive ? 'rgba(20, 5, 5, 0.6)' : 'rgba(10, 10, 10, 0.4)',
            boxShadow: 'inset 0 -1px 0 0 rgba(255,255,255,0.04)',
            backdropFilter: 'blur(24px)',
            WebkitBackdropFilter: 'blur(24px)',
          }}
        >
          {/* Mobile logo */}
          <div className="flex items-center gap-2 md:hidden">
            <div className={`h-8 w-8 rounded-xl flex items-center justify-center shadow-[0_0_16px_rgba(34,211,238,0.4)] ${isIncidentActive ? 'bg-rose-500' : 'bg-cyan-500'}`}>
              <span className="text-[11px] font-black text-[#050505]">RE</span>
            </div>
            <span className="text-xs font-bold tracking-tight font-geist text-[var(--on-surface)]">RedEye</span>
          </div>

          <button
            id="omni-palette-trigger"
            aria-label="Open command palette (Cmd+K)"
            onClick={() => setIsOmniOpen(true)}
            className="hidden md:flex items-center gap-3 px-3 py-1.5 rounded-xl transition-all duration-200 group bg-[var(--surface-container)] text-[var(--on-surface-muted)] hover:text-[var(--on-surface)] border border-white/5"
            style={{ minWidth: '220px' }}
          >
            <Search className={`w-3.5 h-3.5 flex-shrink-0 opacity-70 ${isIncidentActive ? 'text-rose-500' : 'text-[var(--accent-cyan)]'}`} />
            <span className="text-xs font-geist tracking-wide flex-1 text-left">Search commands…</span>
            <span className="flex items-center gap-0.5 text-[9px] font-jetbrains font-bold px-1.5 py-0.5 rounded-md flex-shrink-0 bg-[var(--surface-bright)] text-[var(--on-surface-muted)]">
              <Command className="w-2.5 h-2.5" />K
            </span>
          </button>

          {/* Right controls */}
          <div className="flex items-center gap-1">
            <button
              onClick={toggleTheme}
              className="p-2 rounded-lg transition-all duration-200 text-[var(--text-muted)] hover:text-[var(--accent-cyan)] hover:bg-[var(--surface-container)]"
              aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
            >
              {theme === 'dark' ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
            </button>

            {user && (
              <Link
                to="/dashboard/profile"
                className="hidden sm:flex items-center gap-3 pl-3 ml-1 transition-opacity hover:opacity-80 border-l border-white/5"
              >
                <div className={`h-8 w-8 rounded-full flex items-center justify-center text-xs font-bold font-jetbrains ${isIncidentActive ? 'text-rose-500 bg-rose-500/10' : 'text-cyan-500 bg-cyan-500/10'}`}>
                  {user.email[0].toUpperCase()}
                </div>
                <span className="text-xs font-medium max-w-[120px] truncate tracking-tight font-geist text-[var(--text-muted)]">
                  {user.email}
                </span>
              </Link>
            )}

            <button
              onClick={handleLogout}
              className="p-2 rounded-lg ml-1 transition-all duration-200 text-[var(--text-muted)] hover:text-amber-500 hover:bg-[var(--surface-container)]"
              aria-label="Sign out"
            >
              <LogOut className="w-4 h-4" />
            </button>
          </div>
        </header>

        {/* Mobile nav */}
        <nav
          className="md:hidden overflow-x-auto flex gap-1 px-3 py-2 custom-scrollbar bg-[var(--surface)] shadow-[inset_0_-1px_0_0_rgba(255,255,255,0.04)] backdrop-blur-2xl"
        >
          {NAV_ITEMS.map(({ to, label, icon: Icon, end }) => (
            <NavLink
              key={to}
              to={to}
              end={end}
              className={({ isActive }) =>
                `flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium whitespace-nowrap transition-all font-geist ${
                  isActive ? 'bg-[var(--surface-container)] text-[#22d3ee]' : 'text-[var(--text-muted)]'
                }`
              }
            >
              <Icon className="w-3.5 h-3.5" />
              {label}
            </NavLink>
          ))}
        </nav>

        {/* Content well */}
        <main className="relative flex-1 overflow-y-auto custom-scrollbar z-10">
          <motion.div 
            layout
            animate={{
              padding: isIncidentActive ? '8px' : '32px',
            }}
            className="max-w-[1600px] mx-auto w-full h-full"
          >
            <Outlet />
          </motion.div>
        </main>
      </motion.div>
      
      <OmniPalette isOpen={isOmniOpen} setIsOpen={setIsOmniOpen} />
    </motion.div>
  );
}
