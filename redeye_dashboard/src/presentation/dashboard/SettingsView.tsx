import { useState, useEffect, useCallback, useRef } from 'react';
import {
  Shield,
  Database,
  GitFork,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Settings as SettingsIcon,
  Loader2,
  RefreshCw,
  Sliders,
  ArrowRight,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { Link } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { useToast, type Toast, type ToastVariant } from '../hooks/useToast';
import { BentoCard } from '../components/ui/BentoCard';

// ── Constants ─────────────────────────────────────────────────────────────────

const CONFIG_BASE = 'http://localhost:8085/v1/config';

// ── Styles ────────────────────────────────────────────────────────────────────

const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-[10px] font-bold';
const DATA_CLASS  = 'font-jetbrains text-[var(--on-surface)]';

// ── Domain types ──────────────────────────────────────────────────────────────

interface ClientConfig {
  tenant_id: string;
  pii_masking_enabled: boolean;
  semantic_caching_enabled: boolean;
  routing_fallback_enabled: boolean;
  rate_limit_rpm: number | null;
  preferred_model: string | null;
  updated_at: string;
}

type ToggleKey =
  | 'pii_masking_enabled'
  | 'semantic_caching_enabled'
  | 'routing_fallback_enabled';

type UpdateConfigPayload = Partial<
  Omit<ClientConfig, 'tenant_id' | 'updated_at'>
>;

interface ToggleMeta {
  key: ToggleKey;
  label: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  accentColor: string;
  enabledLabel: string;
  disabledLabel: string;
}

const TOGGLES: ToggleMeta[] = [
  {
    key: 'pii_masking_enabled',
    label: 'PII Masking',
    description:
      'Automatically redacts Personally Identifiable Information (Aadhaar, PAN, SSN, credit cards) before forwarding prompts to LLMs.',
    icon: Shield,
    accentColor: 'var(--accent-cyan)',
    enabledLabel: 'Active',
    disabledLabel: 'Disabled',
  },
  {
    key: 'semantic_caching_enabled',
    label: 'Semantic Cache',
    description:
      'Enables the L2 vector-similarity cache. Semantically equivalent queries hit the cache instead of the model, reducing cost and latency.',
    icon: Database,
    accentColor: 'var(--primary-amber)',
    enabledLabel: 'Active',
    disabledLabel: 'Disabled',
  },
  {
    key: 'routing_fallback_enabled',
    label: 'Routing Fallback',
    description:
      'Automatically hot-swaps to a secondary LLM provider when the primary returns 5xx errors, ensuring uninterrupted service.',
    icon: GitFork,
    accentColor: 'var(--primary-rose)',
    enabledLabel: 'Active',
    disabledLabel: 'Disabled',
  },
];

// ── Framer Motion variants ────────────────────────────────────────────────────

const containerVariants = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.1 }
  }
};

const itemVariants = {
  hidden: { opacity: 0, y: 20 },
  show: { opacity: 1, y: 0, transition: { duration: 0.5, ease: [0.16, 1, 0.3, 1] } }
};

const toastVariant = {
  hidden: { opacity: 0, y: 24, scale: 0.94 },
  show: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.28 } },
  exit: { opacity: 0, y: 12, scale: 0.96, transition: { duration: 0.2 } },
} as const;

// ── Components ────────────────────────────────────────────────────────────────

function ToastList({ toasts, dismiss }: { toasts: Toast[]; dismiss: (id: number) => void }) {
  return (
    <div className="fixed bottom-6 right-6 z-50 flex flex-col gap-3 w-80">
      <AnimatePresence mode="popLayout">
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            layout
            variants={toastVariant}
            initial="hidden"
            animate="show"
            exit="exit"
            className={`flex items-start gap-4 px-5 py-4 rounded-2xl border-none backdrop-blur-xl shadow-2xl ${
              t.variant === 'success' ? 'bg-[rgba(16,185,129,0.1)]' : 
              t.variant === 'error' ? 'bg-[rgba(244,63,94,0.1)]' : 
              'bg-[var(--surface-container-low)]'
            }`}
          >
            <div className="mt-0.5">
              {t.variant === 'success' ? <CheckCircle2 className="w-4 h-4 text-emerald-400" /> : 
               t.variant === 'error' ? <XCircle className="w-4 h-4 text-rose-400" /> : 
               <Shield className="w-4 h-4 text-[var(--accent-cyan)]" />}
            </div>
            <p className="text-xs font-geist text-[var(--on-surface)] flex-1 leading-snug">{t.message}</p>
            <button
              onClick={() => dismiss(t.id)}
              className="text-[var(--text-muted)] hover:text-[var(--on-surface)] transition-colors ml-1"
            >
              ×
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}

interface ToggleCardProps {
  meta: ToggleMeta;
  enabled: boolean;
  isSaving: boolean;
  onToggle: () => void;
}

function ToggleCard({ meta, enabled, isSaving, onToggle }: ToggleCardProps) {
  const { icon: Icon, label, description, accentColor, enabledLabel, disabledLabel } = meta;

  return (
    <BentoCard glowColor={meta.key === 'pii_masking_enabled' ? 'cyan' : meta.key === 'semantic_caching_enabled' ? 'amber' : 'rose'} className="p-8 h-full flex flex-col gap-6">
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-4">
          <div className="p-3 rounded-2xl bg-[var(--surface-bright)] shadow-md">
            <Icon className="w-5 h-5" style={{ color: accentColor }} />
          </div>
          <div>
            <p className="text-sm font-bold text-[var(--on-surface)] font-geist uppercase tracking-tight">{label}</p>
            <p className={`text-[10px] font-bold uppercase tracking-widest mt-1 font-geist ${enabled ? 'text-[var(--accent-cyan)]' : 'text-[var(--text-muted)]'}`}>
              {enabled ? enabledLabel : disabledLabel}
            </p>
          </div>
        </div>

        <button
          id={`toggle-${meta.key}`}
          role="switch"
          aria-checked={enabled}
          disabled={isSaving}
          onClick={onToggle}
          className={`relative inline-flex h-6 w-12 shrink-0 items-center rounded-full transition-all duration-300 ${
            enabled ? 'bg-[var(--surface-bright)]' : 'bg-[var(--surface-container)]'
          }`}
          style={{ boxShadow: enabled ? `inset 0 0 10px ${accentColor}20` : 'none' }}
        >
          <motion.span
            animate={{ x: enabled ? 26 : 4 }}
            className={`inline-block h-5 w-5 rounded-full shadow-lg`}
            style={{ backgroundColor: enabled ? accentColor : 'var(--on-surface-muted)' }}
          />
          {isSaving && (
            <span className="absolute inset-0 flex items-center justify-center">
              <Loader2 className="w-3 h-3 text-white animate-spin" />
            </span>
          )}
        </button>
      </div>

      <p className="text-sm text-[var(--text-muted)] font-geist leading-relaxed">{description}</p>

      <div className="flex items-center gap-2 mt-auto">
        <div className={`w-1.5 h-1.5 rounded-full ${enabled ? 'animate-pulse' : 'opacity-40'}`} style={{ backgroundColor: enabled ? accentColor : 'var(--text-muted)', boxShadow: enabled ? `0 0 10px ${accentColor}` : 'none' }} />
        <span className={`${LABEL_CLASS} tracking-widest`}>
          {enabled ? 'Enforced gateway-wide' : 'Policy idle'}
        </span>
      </div>
    </BentoCard>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export function SettingsView() {
  const { user } = useAuth();
  const tenantId = user?.tenantId ?? 'default_tenant';

  const [config, setConfig] = useState<ClientConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [savingKey, setSavingKey] = useState<ToggleKey | null>(null);

  const { toasts, push, dismiss } = useToast();

  const fetchConfig = useCallback(async () => {
    setIsLoading(true);
    setFetchError(null);
    try {
      const res = await fetch(`${CONFIG_BASE}/${tenantId}`, {
        credentials: 'include',
        headers: { Accept: 'application/json', 'x-csrf-token': '1' },
      });
      if (!res.ok) {
        if (res.status === 404) {
          setConfig({
            tenant_id: tenantId,
            pii_masking_enabled: false,
            semantic_caching_enabled: false,
            routing_fallback_enabled: false,
            rate_limit_rpm: null,
            preferred_model: null,
            updated_at: new Date().toISOString(),
          });
          return;
        }
        throw new Error(`HTTP ${res.status}`);
      }
      const data: ClientConfig = await res.json();
      setConfig(data);
    } catch (err) {
      setFetchError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  }, [tenantId]);

  useEffect(() => {
    void fetchConfig();
  }, [fetchConfig]);

  const handleToggle = useCallback(
    async (key: ToggleKey) => {
      if (!config || savingKey !== null) return;
      const nextValue = !config[key];
      setConfig((prev) => (prev ? { ...prev, [key]: nextValue } : prev));
      setSavingKey(key);
      const payload: UpdateConfigPayload = { [key]: nextValue };

      try {
        const res = await fetch(`${CONFIG_BASE}/${tenantId}`, {
          method: 'PUT',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json', Accept: 'application/json', 'x-csrf-token': '1' },
          body: JSON.stringify(payload),
        });

        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const saved: ClientConfig = await res.json();
        setConfig(saved);
        const meta = TOGGLES.find((t) => t.key === key);
        push(`${meta?.label ?? key} ${nextValue ? 'enabled' : 'disabled'} successfully.`, 'success');
      } catch (err) {
        setConfig((prev) => prev ? { ...prev, [key]: !nextValue } : prev);
        push(`Failed to update setting: ${err instanceof Error ? err.message : 'Save failed'}`, 'error');
      } finally {
        setSavingKey(null);
      }
    },
    [config, savingKey, tenantId, push],
  );

  if (isLoading && !config) {
    return (
      <div className="grid grid-cols-12 gap-6 p-6 animate-pulse">
        <div className="col-span-12 h-24 bg-[var(--surface-container)] rounded-2xl" />
        <div className="col-span-4 h-48 bg-[var(--surface-container)] rounded-2xl" />
        <div className="col-span-4 h-48 bg-[var(--surface-container)] rounded-2xl" />
        <div className="col-span-4 h-48 bg-[var(--surface-container)] rounded-2xl" />
      </div>
    );
  }

  return (
    <>
      <motion.div
        variants={containerVariants}
        initial="hidden"
        animate="show"
        className="grid grid-cols-12 gap-6 p-6 auto-rows-max text-[var(--on-surface)]"
      >
        {/* Breadcrumb */}
        <motion.div variants={itemVariants} className="col-span-12 flex items-center gap-3 text-sm font-mono text-[var(--text-muted)] mb-2">
          <Link to="/dashboard" className="hover:text-[var(--on-surface)] transition-colors flex items-center gap-2 font-geist tracking-wide">
            <Shield className="w-4 h-4" />
            Dashboard
          </Link>
          <ArrowRight className="w-4 h-4" />
          <span className="text-[var(--on-surface)] font-geist">Settings</span>
        </motion.div>

        {/* Header */}
        <motion.header variants={itemVariants} className="col-span-12 flex flex-col md:flex-row md:items-end justify-between gap-6 mb-8">
          <div>
            <p className={`${LABEL_CLASS} text-[var(--accent-cyan)] mb-1`}>Control Plane</p>
            <h1 className="text-4xl font-extrabold tracking-tight text-[var(--on-surface)] mb-2 font-geist">
              Gateway Settings
            </h1>
            <p className="text-sm text-[var(--text-muted)] max-w-2xl font-geist">
              Real-time feature flags synced to the gateway via Redis. Changes propagate in under 1ms.
            </p>
          </div>
          
          <div className="flex items-center gap-3 p-4 rounded-2xl bg-[var(--surface-bright)] shadow-md">
            <div className={`w-2 h-2 rounded-full ${config ? 'bg-[var(--accent-cyan)] shadow-[0_0_10px_var(--accent-cyan)]' : 'bg-[var(--text-muted)]'} animate-pulse`} />
            <span className={LABEL_CLASS}>{config ? 'Config Loaded' : 'No Sync'}</span>
          </div>
        </motion.header>

        {fetchError && (
          <motion.div variants={itemVariants} className="col-span-12 p-6 rounded-2xl bg-[rgba(244,63,94,0.1)] flex items-center justify-between gap-4">
            <div className="flex items-center gap-4">
              <AlertTriangle className="w-6 h-6 text-rose-400" />
              <div>
                <p className="text-sm font-bold text-rose-400 font-geist uppercase tracking-tight">Sync Failure</p>
                <p className="text-xs text-[var(--text-muted)] mt-1 font-geist">{fetchError}</p>
              </div>
            </div>
            <button onClick={() => void fetchConfig()} className="px-4 py-2 rounded-xl bg-[var(--surface-bright)] text-xs font-bold font-geist uppercase tracking-widest hover:bg-white/10 transition-all flex items-center gap-2">
              <RefreshCw className="w-3 h-3" /> Retry
            </button>
          </motion.div>
        )}

        {/* Feature Toggles */}
        {config && TOGGLES.map((meta) => (
          <motion.div key={meta.key} variants={itemVariants} className="col-span-12 lg:col-span-4 h-[240px]">
            <ToggleCard
              meta={meta}
              enabled={config[meta.key]}
              isSaving={savingKey === meta.key}
              onToggle={() => void handleToggle(meta.key)}
            />
          </motion.div>
        ))}

        {/* Advanced Config */}
        {config && (
          <motion.div variants={itemVariants} className="col-span-12">
            <BentoCard glowColor="none" className="p-8">
              <div className="flex items-center gap-3 mb-8">
                <div className="p-3 rounded-2xl bg-[var(--surface-bright)] shadow-md">
                  <Sliders className="w-5 h-5 text-[var(--accent-cyan)]" />
                </div>
                <h2 className="text-xl font-bold font-geist">Advanced Configuration</h2>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
                <div>
                  <label className={`${LABEL_CLASS} block mb-3`}>Rate Limit (req/min)</label>
                  <div className="relative">
                    <input
                      type="text"
                      readOnly
                      value={config.rate_limit_rpm ?? 'Gateway Default'}
                      className="w-full h-14 px-6 rounded-2xl bg-[var(--surface-container)] text-[var(--on-surface)] font-jetbrains text-sm font-bold border-none focus:ring-0 cursor-not-allowed opacity-60"
                    />
                    <div className="absolute right-6 top-1/2 -translate-y-1/2 flex items-center gap-2">
                       <span className="text-[10px] uppercase font-bold text-[var(--text-muted)] font-geist">Read Only</span>
                    </div>
                  </div>
                </div>

                <div>
                  <label className={`${LABEL_CLASS} block mb-3`}>Preferred Model</label>
                  <div className="relative">
                    <input
                      type="text"
                      readOnly
                      value={config.preferred_model ?? 'Auto-negotiate'}
                      className="w-full h-14 px-6 rounded-2xl bg-[var(--surface-container)] text-[var(--on-surface)] font-jetbrains text-sm font-bold border-none focus:ring-0 cursor-not-allowed opacity-60"
                    />
                    <div className="absolute right-6 top-1/2 -translate-y-1/2">
                       <span className="text-[10px] uppercase font-bold text-[var(--text-muted)] font-geist tracking-widest">Locked</span>
                    </div>
                  </div>
                </div>
              </div>

              <div className="mt-8 pt-8 border-t border-[var(--surface-bright)] flex items-center justify-between">
                <p className="text-[10px] text-[var(--text-muted)] font-geist flex items-center gap-2 uppercase tracking-[0.2em] font-bold">
                  <SettingsIcon className="w-3 h-3" />
                  Controlled via redeye_config API
                </p>
                <p className={`${DATA_CLASS} text-[10px] opacity-60`}>
                  Last Synced: {new Date(config.updated_at).toLocaleTimeString()}
                </p>
              </div>
            </BentoCard>
          </motion.div>
        )}
      </motion.div>

      <ToastList toasts={toasts} dismiss={dismiss} />
    </>
  );
}
