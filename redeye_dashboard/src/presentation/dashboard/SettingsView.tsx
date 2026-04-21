// Dashboard View — SettingsView
// Fetches per-tenant feature-flag config from redeye_config (port 8085),
// renders toggles for PII Masking, Semantic Cache, and Routing Fallback,
// and persists changes via PUT with per-toggle optimistic UX feedback.
//
// Engineering constraints observed:
//  • Zero `any` types — every API shape has an explicit interface.
//  • All fetch states (isLoading, isError, isSaving per toggle) are explicit.
//  • Toggle is disabled + shows an inline spinner while its own PUT is in-flight.
//  • Toast notification confirms success or surfaces the error after each save.

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
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useAuth } from '../context/AuthContext';
import { useToast, type Toast, type ToastVariant } from '../hooks/useToast';

// ── Constants ─────────────────────────────────────────────────────────────────

const CONFIG_BASE = 'http://localhost:8085/v1/config';

// ── Domain types ──────────────────────────────────────────────────────────────

/** Exact mirror of the Rust `ClientConfig` struct returned by redeye_config. */
interface ClientConfig {
  tenant_id: string;
  pii_masking_enabled: boolean;
  semantic_caching_enabled: boolean;
  routing_fallback_enabled: boolean;
  rate_limit_rpm: number | null;
  preferred_model: string | null;
  updated_at: string;
}

/** Keys of ClientConfig that map to boolean toggles in this view. */
type ToggleKey =
  | 'pii_masking_enabled'
  | 'semantic_caching_enabled'
  | 'routing_fallback_enabled';

/** Partial update payload sent to PUT /v1/config/:tenant_id */
type UpdateConfigPayload = Partial<
  Omit<ClientConfig, 'tenant_id' | 'updated_at'>
>;

// ── Toggle metadata ───────────────────────────────────────────────────────────

interface ToggleMeta {
  key: ToggleKey;
  label: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  accentClass: string;
  enabledLabel: string;
  disabledLabel: string;
}

const TOGGLES: ToggleMeta[] = [
  {
    key: 'pii_masking_enabled',
    label: 'PII Masking',
    description:
      'Automatically redacts Personally Identifiable Information (Aadhaar, PAN, SSN, credit cards) before forwarding prompts to upstream LLMs.',
    icon: Shield,
    accentClass: 'from-cyan-500 to-teal-500',
    enabledLabel: 'Active',
    disabledLabel: 'Disabled',
  },
  {
    key: 'semantic_caching_enabled',
    label: 'Semantic Cache',
    description:
      'Enables the L2 vector-similarity cache. Semantically equivalent queries hit the cache instead of the upstream LLM, reducing cost and latency.',
    icon: Database,
    accentClass: 'from-violet-500 to-purple-500',
    enabledLabel: 'Active',
    disabledLabel: 'Disabled',
  },
  {
    key: 'routing_fallback_enabled',
    label: 'Routing Fallback',
    description:
      'Automatically hot-swaps to a secondary LLM provider when the primary returns 5xx errors, ensuring uninterrupted service.',
    icon: GitFork,
    accentClass: 'from-amber-500 to-orange-500',
    enabledLabel: 'Active',
    disabledLabel: 'Disabled',
  },
];

// ── Framer Motion variants ────────────────────────────────────────────────────

const containerVariants = {
  hidden: {},
  show: { transition: { staggerChildren: 0.07 } },
} as const;

const fadeUpVariant = {
  hidden: { opacity: 0, y: 18 },
  show: {
    opacity: 1,
    y: 0,
    transition: {
      duration: 0.42,
      ease: [0.25, 0.1, 0.25, 1] as [number, number, number, number],
    },
  },
} as const;

const toastVariant = {
  hidden: { opacity: 0, y: 24, scale: 0.94 },
  show: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.28 } },
  exit: { opacity: 0, y: 12, scale: 0.96, transition: { duration: 0.2 } },
} as const;

// ── Toast renderer ────────────────────────────────────────────────────────────

function toastIcon(variant: ToastVariant) {
  if (variant === 'success') return <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />;
  if (variant === 'error')   return <XCircle      className="w-4 h-4 text-rose-400 shrink-0" />;
  return                            <Shield       className="w-4 h-4 text-cyan-400 shrink-0" />;
}

function toastBg(variant: ToastVariant) {
  if (variant === 'success') return 'border-emerald-500/30 bg-emerald-500/10';
  if (variant === 'error')   return 'border-rose-500/30 bg-rose-500/10';
  return                            'border-cyan-500/30 bg-cyan-500/10';
}

function ToastList({ toasts, dismiss }: { toasts: Toast[]; dismiss: (id: number) => void }) {
  return (
    <div
      role="region"
      aria-live="polite"
      aria-label="Notifications"
      className="fixed bottom-6 right-6 z-50 flex flex-col gap-2 w-80"
    >
      <AnimatePresence mode="popLayout">
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            layout
            variants={toastVariant}
            initial="hidden"
            animate="show"
            exit="exit"
            role="alert"
            className={`flex items-start gap-3 px-4 py-3 rounded-xl border backdrop-blur-xl shadow-xl ${toastBg(t.variant)}`}
          >
            {toastIcon(t.variant)}
            <p className="text-sm text-slate-200 flex-1 leading-snug">{t.message}</p>
            <button
              onClick={() => dismiss(t.id)}
              aria-label="Dismiss notification"
              className="text-slate-500 hover:text-slate-300 transition-colors ml-1 mt-0.5"
            >
              ×
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}

// ── Toggle Card ───────────────────────────────────────────────────────────────

interface ToggleCardProps {
  meta: ToggleMeta;
  enabled: boolean;
  isSaving: boolean;
  onToggle: () => void;
}

function ToggleCard({ meta, enabled, isSaving, onToggle }: ToggleCardProps) {
  const { icon: Icon, label, description, accentClass, enabledLabel, disabledLabel } = meta;

  return (
    <motion.div
      variants={fadeUpVariant}
      className="glass-panel p-5 sm:p-6 flex flex-col gap-4"
    >
      {/* Header row */}
      <div className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-3 min-w-0">
          {/* Icon badge */}
          <div
            className={`p-2.5 rounded-xl bg-gradient-to-br ${accentClass} shadow-lg shrink-0`}
          >
            <Icon className="w-4 h-4 text-white" />
          </div>
          <div className="min-w-0">
            <p className="text-sm font-bold text-slate-100 leading-tight">{label}</p>
            <p
              className={`text-[11px] font-semibold mt-0.5 ${
                enabled ? 'text-emerald-400' : 'text-slate-500'
              }`}
            >
              {enabled ? enabledLabel : disabledLabel}
            </p>
          </div>
        </div>

        {/* Toggle switch */}
        <button
          id={`toggle-${meta.key}`}
          role="switch"
          aria-checked={enabled}
          aria-label={`${label} — ${enabled ? 'disable' : 'enable'}`}
          disabled={isSaving}
          onClick={onToggle}
          className={`relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors duration-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-400 focus-visible:ring-offset-2 focus-visible:ring-offset-slate-900 disabled:cursor-not-allowed disabled:opacity-60 ${
            enabled ? 'bg-gradient-to-r ' + accentClass : 'bg-slate-700'
          }`}
        >
          <span
            className={`inline-block h-4.5 w-4.5 transform rounded-full bg-white shadow-md transition-transform duration-300 ${
              enabled ? 'translate-x-5' : 'translate-x-1'
            }`}
          />
          {/* Per-toggle saving spinner overlaid on the thumb */}
          {isSaving && (
            <span className="absolute inset-0 flex items-center justify-center">
              <Loader2 className="w-3 h-3 text-white animate-spin" />
            </span>
          )}
        </button>
      </div>

      {/* Description */}
      <p className="text-xs text-slate-400 leading-relaxed">{description}</p>

      {/* Status pill */}
      <div className="flex items-center gap-2">
        <div
          className={`w-1.5 h-1.5 rounded-full ${
            enabled ? 'bg-emerald-500 neon-dot' : 'bg-slate-600'
          }`}
        />
        <span className="text-[11px] uppercase tracking-widest font-medium text-slate-500">
          {enabled ? 'Enforced gateway-wide' : 'Not enforced'}
        </span>
      </div>
    </motion.div>
  );
}

// ── Loading skeleton ──────────────────────────────────────────────────────────

function LoadingSkeleton() {
  return (
    <div className="space-y-6 animate-pulse">
      <header>
        <div className="w-24 h-2.5 bg-slate-800/70 rounded mb-3" />
        <div className="w-64 h-8 bg-slate-800/70 rounded mb-2" />
        <div className="w-80 h-3.5 bg-slate-800/50 rounded" />
      </header>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {[0, 1, 2].map((i) => (
          <div key={i} className="h-36 glass-panel bg-slate-900/40" />
        ))}
      </div>
      <div className="h-28 glass-panel bg-slate-900/40" />
    </div>
  );
}

// ── Error banner ──────────────────────────────────────────────────────────────

function ErrorBanner({ message, onRetry }: { message: string; onRetry: () => void }) {
  return (
    <div className="glass-panel p-5 border-rose-500/30 bg-rose-500/5 flex items-start gap-3">
      <AlertTriangle className="w-5 h-5 text-rose-400 shrink-0 mt-0.5" />
      <div className="flex-1">
        <p className="text-sm font-semibold text-rose-300">Failed to load configuration</p>
        <p className="text-xs text-slate-400 mt-1">{message}</p>
      </div>
      <button
        onClick={onRetry}
        className="flex items-center gap-1.5 text-xs font-semibold text-rose-400 hover:text-rose-200 transition-colors px-3 py-1.5 rounded-lg border border-rose-500/30 hover:bg-rose-500/10"
      >
        <RefreshCw className="w-3 h-3" />
        Retry
      </button>
    </div>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export function SettingsView() {
  const { user } = useAuth();
  // Use tenant_id from the authenticated user; fall back to "default_tenant"
  // for local development where auth may be bypassed.
  const tenantId = user?.tenantId ?? 'default_tenant';

  // ── Data state ─────────────────────────────────────────────────────────────
  const [config, setConfig] = useState<ClientConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [fetchError, setFetchError] = useState<string | null>(null);

  // Tracks which toggle key is currently mid-PUT.
  const [savingKey, setSavingKey] = useState<ToggleKey | null>(null);

  // Stable fetch reference so the retry button doesn't need extra plumbing.
  const fetchRef = useRef<() => Promise<void>>();

  const { toasts, push, dismiss } = useToast();

  // ── Fetch config on mount (and on retry) ───────────────────────────────────
  const fetchConfig = useCallback(async () => {
    setIsLoading(true);
    setFetchError(null);
    try {
      const res = await fetch(`${CONFIG_BASE}/${tenantId}`, {
        credentials: 'include',
        headers: { Accept: 'application/json', 'x-csrf-token': '1' },
      });
      if (!res.ok) {
        const body: unknown = await res.json().catch(() => ({}));
        const msg =
          body !== null &&
          typeof body === 'object' &&
          'error' in body &&
          body.error !== null &&
          typeof body.error === 'object' &&
          'message' in body.error &&
          typeof body.error.message === 'string'
            ? body.error.message
            : `HTTP ${res.status}`;
        throw new Error(msg);
      }
      const data: ClientConfig = await res.json();
      setConfig(data);
    } catch (err) {
      setFetchError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsLoading(false);
    }
  }, [tenantId]);

  // Store the latest ref so retry can call it without stale closure.
  fetchRef.current = fetchConfig;

  useEffect(() => {
    void fetchRef.current?.();
  }, [fetchConfig]);

  // ── Toggle handler — per-key optimistic save ───────────────────────────────
  const handleToggle = useCallback(
    async (key: ToggleKey) => {
      if (!config || savingKey !== null) return;

      const nextValue = !config[key];

      // Optimistic update for instant visual feedback.
      setConfig((prev) => (prev ? { ...prev, [key]: nextValue } : prev));
      setSavingKey(key);

      const payload: UpdateConfigPayload = { [key]: nextValue };

      try {
        const res = await fetch(`${CONFIG_BASE}/${tenantId}`, {
          method: 'PUT',
          credentials: 'include',
          headers: {
            'Content-Type': 'application/json',
            Accept: 'application/json',
            'x-csrf-token': '1',
          },
          body: JSON.stringify(payload),
        });

        if (!res.ok) {
          const body: unknown = await res.json().catch(() => ({}));
          const msg =
            body !== null &&
            typeof body === 'object' &&
            'error' in body &&
            body.error !== null &&
            typeof body.error === 'object' &&
            'message' in body.error &&
            typeof body.error.message === 'string'
              ? body.error.message
              : `HTTP ${res.status}`;
          throw new Error(msg);
        }

        // Sync with server's authoritative response (RETURNING * from Postgres).
        const saved: ClientConfig = await res.json();
        setConfig(saved);

        const meta = TOGGLES.find((t) => t.key === key);
        const label = meta?.label ?? key;
        push(`${label} ${nextValue ? 'enabled' : 'disabled'} successfully.`, 'success');
      } catch (err) {
        // Roll back the optimistic change on failure.
        setConfig((prev) =>
          prev ? { ...prev, [key]: !nextValue } : prev,
        );
        const msg = err instanceof Error ? err.message : 'Save failed';
        push(`Failed to update setting: ${msg}`, 'error');
      } finally {
        setSavingKey(null);
      }
    },
    [config, savingKey, tenantId, push],
  );

  // ── Render ─────────────────────────────────────────────────────────────────

  if (isLoading) return <LoadingSkeleton />;

  return (
    <>
      <motion.div
        variants={containerVariants}
        initial="hidden"
        animate="show"
        className="space-y-6"
      >
        {/* ── Page Header ─────────────────────────────────────────────── */}
        <motion.header variants={fadeUpVariant} className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-[0.25em] text-slate-500 mb-1 font-medium">
              Control Plane
            </p>
            <h1 className="text-2xl sm:text-3xl lg:text-4xl font-extrabold tracking-tight bg-gradient-to-r from-cyan-400 to-teal-400 bg-clip-text text-transparent pb-1">
              Gateway Settings
            </h1>
            <p className="text-xs sm:text-sm text-slate-400 mt-1">
              Real-time feature flags synced to the gateway via Redis.
              Changes propagate in under 1&nbsp;ms.
            </p>
          </div>

          <div className="flex items-center gap-2 glass-panel px-3 py-1.5 rounded-full w-fit">
            <div className={`w-2 h-2 rounded-full ${config ? 'bg-emerald-500 neon-dot' : 'bg-slate-600'}`} />
            <span className="text-xs font-medium text-slate-300">
              {config ? 'Config loaded' : 'No config'}
            </span>
          </div>
        </motion.header>

        {/* ── Error banner ─────────────────────────────────────────────── */}
        {fetchError && (
          <motion.div variants={fadeUpVariant}>
            <ErrorBanner
              message={fetchError}
              onRetry={() => void fetchConfig()}
            />
          </motion.div>
        )}

        {/* ── Feature Toggle Cards ─────────────────────────────────────── */}
        {config && (
          <>
            <motion.div
              variants={fadeUpVariant}
              className="grid grid-cols-1 md:grid-cols-3 gap-4 sm:gap-5"
            >
              {TOGGLES.map((meta) => (
                <ToggleCard
                  key={meta.key}
                  meta={meta}
                  enabled={config[meta.key]}
                  isSaving={savingKey === meta.key}
                  onToggle={() => void handleToggle(meta.key)}
                />
              ))}
            </motion.div>

            {/* ── Advanced Settings Panel ──────────────────────────────── */}
            <motion.div variants={fadeUpVariant} className="glass-panel p-5 sm:p-6">
              <div className="flex items-center gap-2 mb-5">
                <Sliders className="w-4 h-4 text-cyan-400" />
                <h2 className="text-sm font-bold text-slate-100">Advanced Configuration</h2>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {/* Rate limit RPM */}
                <div>
                  <label
                    htmlFor="rate-limit-rpm"
                    className="block text-xs font-semibold text-slate-400 mb-1.5 uppercase tracking-wider"
                  >
                    Rate Limit (req / min)
                  </label>
                  <div className="flex items-center gap-2">
                    <input
                      id="rate-limit-rpm"
                      type="number"
                      min={1}
                      placeholder="Gateway default"
                      value={config.rate_limit_rpm ?? ''}
                      readOnly
                      className="premium-input flex-1 cursor-not-allowed opacity-60"
                      title="Edit via API — PUT /v1/config/:tenant_id { rate_limit_rpm }"
                    />
                    <span className="text-xs text-slate-500 whitespace-nowrap">
                      {config.rate_limit_rpm ? `${config.rate_limit_rpm} rpm` : 'default'}
                    </span>
                  </div>
                </div>

                {/* Preferred model */}
                <div>
                  <label
                    htmlFor="preferred-model"
                    className="block text-xs font-semibold text-slate-400 mb-1.5 uppercase tracking-wider"
                  >
                    Preferred Model
                  </label>
                  <input
                    id="preferred-model"
                    type="text"
                    placeholder="Provider default"
                    value={config.preferred_model ?? ''}
                    readOnly
                    className="premium-input w-full cursor-not-allowed opacity-60"
                    title="Edit via API — PUT /v1/config/:tenant_id { preferred_model }"
                  />
                </div>
              </div>

              <p className="text-[11px] text-slate-600 mt-4 flex items-center gap-1.5">
                <SettingsIcon className="w-3 h-3" />
                Advanced fields are managed via the{' '}
                <code className="text-cyan-400 font-mono">redeye_config</code> API.
                Last updated:{' '}
                {new Date(config.updated_at).toLocaleString()}
              </p>
            </motion.div>
          </>
        )}
      </motion.div>

      {/* ── Toast notifications ──────────────────────────────────────────── */}
      <ToastList toasts={toasts} dismiss={dismiss} />
    </>
  );
}
