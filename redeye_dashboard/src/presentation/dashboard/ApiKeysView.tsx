import { useState, useCallback } from 'react';
import {
  Key,
  Plus,
  Trash2,
  X,
  AlertTriangle,
  ShieldCheck,
  Copy,
  Check,
  Globe,
  Loader2,
  Lock,
  Server,
  User,
  Users,
  ArrowRight,
  Sparkles,
  Shield,
  CheckCircle2,
  XCircle,
} from 'lucide-react';
import useSWR from 'swr';
import { useAuth } from '../context/AuthContext';
import { motion, AnimatePresence } from 'framer-motion';
import { Link } from 'react-router-dom';

import { useToast, type Toast, type ToastVariant } from '../hooks/useToast';
import { BentoCard } from '../components/ui/BentoCard';
import { InlineSparkline } from '../components/ui/InlineSparkline';
import { QuotaRadialBar } from '../components/ui/QuotaRadialBar';
import { KeyUsageHeatmap } from '../components/ui/KeyUsageHeatmap';

// ── Constants ─────────────────────────────────────────────────────────────────

const AUTH_BASE = 'http://localhost:8084/v1/auth';
const CONFIG_BASE = 'http://localhost:8085/v1/config';

// ── Domain types ──────────────────────────────────────────────────────────────

export interface ApiKey {
  id: string;
  name: string;
  created_at: string;
  expires_at: string | null;
  is_active: boolean;
  daily_requests?: number[];
  quota_used?: number;
  model_name?: string;
}

interface ProviderKey {
  id: string;
  provider_name: string;
  created_at: string;
}

// ── SWR fetcher ───────────────────────────────────────────────────────────────

const fetcher = async (url: string): Promise<unknown> => {
  const res = await fetch(url, {
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' },
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
};

// ── Formatter & Styles ────────────────────────────────────────────────────────

const LABEL_CLASS = 'font-geist text-[var(--on-surface-muted)] uppercase tracking-widest text-xs font-bold';
const DATA_CLASS = 'font-jetbrains text-[var(--on-surface)]';

// ── Skeuomorphism 2.0 Tokens ──────────────────────────────────────────────────

const TACTILE_CARD = `
  relative overflow-hidden
  bg-gradient-to-br from-[var(--surface-container)] to-[var(--bg-canvas)]
  border-t border-l border-[rgba(255,255,255,0.12)]
  border-b-2 border-r-2 border-[rgba(0,0,0,0.45)]
  shadow-[0_12px_32px_-8px_rgba(0,0,0,0.5),inset_0_1px_1px_rgba(255,255,255,0.05)]
`;

const MECHANICAL_BTN = `
  relative inline-flex items-center justify-center
  bg-[var(--surface-bright)] text-[var(--on-surface)]
  border-t border-l border-[rgba(255,255,255,0.1)]
  border-b-[3px] border-r-[1px] border-[rgba(0,0,0,0.4)]
  shadow-[0_4px_12px_-2px_rgba(0,0,0,0.3)]
  active:translate-y-[3px] active:border-b-[1px] active:shadow-[inset_0_2px_4px_rgba(0,0,0,0.4)]
  transition-all duration-75 select-none
`;

const ENGRAVED_TEXT = `
  [text-shadow:0px_1px_1px_rgba(255,255,255,0.12)]
  color-[var(--on-surface-muted)]
`;

const INDUSTRIAL_SLOT = `
  border-2 border-dashed border-[var(--surface-bright)]
  bg-[rgba(0,0,0,0.15)]
  shadow-[inset_0_6px_16px_rgba(0,0,0,0.4)]
  rounded-3xl flex flex-col items-center justify-center
`;

function fmtMag(raw: string | number | undefined | null): string {
  if (raw === undefined || raw === null) return '—';
  const n = typeof raw === 'string' ? parseFloat(raw) : raw;
  if (isNaN(n)) return '—';
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`;
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toFixed(n % 1 === 0 ? 0 : 1);
}

// ── Framer Motion variants ────────────────────────────────────────────────────

const cardVariants = {
  hidden: { opacity: 0, y: 20, scale: 0.95 },
  visible: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.4, ease: [0.25, 0.1, 0.25, 1] as const } },
  hover: { scale: 1.02, transition: { duration: 0.2 } },
};

const containerVariants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: { staggerChildren: 0.1, delayChildren: 0.1 } },
};

const modalVariants = {
  hidden: { opacity: 0, scale: 0.9 },
  visible: { opacity: 1, scale: 1, transition: { duration: 0.3, ease: [0.25, 0.1, 0.25, 1] as const } },
  exit: { opacity: 0, scale: 0.9, transition: { duration: 0.2 } },
};

const toastItemVariant = {
  hidden: { opacity: 0, y: 24, scale: 0.94 },
  show: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.28 } },
  exit: { opacity: 0, y: 12, scale: 0.96, transition: { duration: 0.2 } },
} as const;

// ── Components ──────────────────────────────────────────────────────────────

function StatusIndicator({ isActive, quotaUsed }: { isActive: boolean; quotaUsed: number }) {
  if (!isActive) return <div className="w-2.5 h-2.5 rounded-full bg-rose-500 shadow-[0_0_10px_rgba(244,63,94,0.8)]" title="Revoked" />;
  if (quotaUsed > 80) return <div className="w-2.5 h-2.5 rounded-full bg-amber-500 shadow-[0_0_10px_rgba(245,158,11,0.8)]" title="Approaching Limit" />;
  return <div className="w-2.5 h-2.5 rounded-full bg-[var(--accent-cyan)] shadow-[0_0_10px_rgba(34,211,238,0.8)] animate-pulse" title="Active" />;
}

// ── Toast renderer ────────────────────────────────────────────────────────────

function toastIcon(variant: ToastVariant) {
  if (variant === 'success') return <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />;
  if (variant === 'error') return <XCircle className="w-4 h-4 text-rose-400 shrink-0" />;
  return <Shield className="w-4 h-4 text-cyan-400 shrink-0" />;
}

function toastBg(variant: ToastVariant) {
  // No-line rule: replace borders with tonal shifts
  if (variant === 'success') return 'bg-[rgba(16,185,129,0.1)]';
  if (variant === 'error') return 'bg-[rgba(244,63,94,0.1)]';
  return 'bg-[rgba(34,211,238,0.1)]';
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
            variants={toastItemVariant}
            initial="hidden"
            animate="show"
            exit="exit"
            role="alert"
            className={`flex items-start gap-3 px-4 py-3 rounded-xl backdrop-blur-xl shadow-xl ${toastBg(t.variant)}`}
          >
            {toastIcon(t.variant)}
            <p className="text-sm text-[var(--on-surface)] flex-1 leading-snug">{t.message}</p>
            <button
              onClick={() => dismiss(t.id)}
              aria-label="Dismiss notification"
              className="text-[var(--text-muted)] hover:text-[var(--on-surface)] transition-colors ml-1 mt-0.5"
            >
              <X className="w-4 h-4" />
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}

// ── Revoke Confirmation Modal ─────────────────────────────────────────────────

interface RevokeModalProps {
  keyToRevoke: ApiKey;
  isRevoking: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

function RevokeConfirmModal({ keyToRevoke, isRevoking, onConfirm, onCancel }: RevokeModalProps) {
  return (
    <motion.div
      variants={modalVariants}
      initial="hidden"
      animate="visible"
      exit="exit"
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-[var(--bg-canvas)]/80 backdrop-blur-md"
      role="dialog"
      aria-modal="true"
      aria-labelledby="revoke-modal-title"
    >
      <motion.div
        variants={modalVariants}
        className="bg-[var(--surface-container)] shadow-2xl shadow-[var(--primary-rose)]/20 rounded-3xl w-full max-w-md overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-5 bg-[rgba(244,63,94,0.1)]">
          <h3
            id="revoke-modal-title"
            className="text-lg font-bold text-[var(--on-surface)] flex items-center gap-2 font-geist"
          >
            <AlertTriangle className="w-5 h-5 text-[var(--primary-rose)]" />
            Revoke API Key
          </h3>
          <button
            onClick={onCancel}
            disabled={isRevoking}
            className="text-[var(--text-muted)] hover:text-[var(--on-surface)] transition-colors p-1 disabled:opacity-50"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="p-6 space-y-5">
          <div className="flex items-start gap-3 p-4 rounded-2xl bg-[rgba(244,63,94,0.1)]">
            <AlertTriangle className="w-4 h-4 text-[var(--primary-rose)] shrink-0 mt-0.5" />
            <p className="text-sm text-[var(--on-surface)]">
              This action is <strong>permanent and irreversible</strong>. Any
              service or integration using this key will immediately lose access.
            </p>
          </div>

          <div className="rounded-2xl bg-[rgba(255,255,255,0.02)] p-4">
            <p className={`${LABEL_CLASS} mb-2`}>
              Key to revoke
            </p>
            <div className="flex items-center gap-3">
              <Key className="w-5 h-5 text-[var(--primary-rose)] shrink-0" />
              <span className={`text-base font-bold ${DATA_CLASS}`}>{keyToRevoke.name}</span>
            </div>
            <p className={`${DATA_CLASS} text-[10px] text-[var(--text-muted)] mt-2`}>
              Created {new Date(keyToRevoke.created_at).toLocaleDateString()}
            </p>
          </div>
        </div>

        <div className="flex items-center justify-end gap-3 px-6 py-5">
          <button
            type="button"
            onClick={onCancel}
            disabled={isRevoking}
            className="rounded-xl px-5 py-2.5 text-sm font-semibold text-[var(--text-muted)] hover:text-[var(--on-surface)] hover:bg-[rgba(255,255,255,0.05)] transition-colors disabled:opacity-50 font-geist"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={isRevoking}
            className="inline-flex items-center gap-2 rounded-xl px-5 py-2.5 text-sm font-bold text-[var(--on-surface)] font-geist uppercase tracking-widest bg-gradient-to-br from-[var(--primary-amber)] to-[var(--primary-rose)] text-black shadow-[0_4px_0_0_rgba(160,40,10,0.6),0_8px_24px_-4px_rgba(251,191,36,0.4)] hover:shadow-[0_4px_0_0_rgba(160,40,10,0.6),0_14px_32px_-4px_rgba(251,191,36,0.55)] active:translate-y-[2px] transition-all disabled:opacity-60 disabled:cursor-not-allowed"
          >
            {isRevoking ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Revoking…
              </>
            ) : (
              <>
                <Trash2 className="w-4 h-4" />
                Yes, Revoke Key
              </>
            )}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}

// ── Main component ────────────────────────────────────────────────────────────

export function ApiKeysView() {
  const { user } = useAuth();
  const accountType = user?.accountType ?? 'individual';
  const isTeam = accountType === 'team';
  const tenantId = user?.tenantId ?? 'default_tenant';

  const { toasts, push, dismiss } = useToast();

  const {
    data: keys,
    error: keysError,
    isLoading: keysLoading,
    mutate: mutateKeys,
  } = useSWR<ApiKey[]>(
    isTeam ? `${CONFIG_BASE}/${tenantId}/api-keys` : null,
    fetcher as (url: string) => Promise<ApiKey[]>,
  );

  const {
    data: providerKeys,
    error: providerError,
    isLoading: providerLoading,
    mutate: mutateProviders,
  } = useSWR<ProviderKey[]>(
    `${AUTH_BASE}/provider-keys`,
    fetcher as (url: string) => Promise<ProviderKey[]>,
  );

  const [isProviderModalOpen, setIsProviderModalOpen] = useState(false);
  const [isKeyModalOpen, setIsKeyModalOpen] = useState(false);
  const [newProviderName, setNewProviderName] = useState('openai');
  const [newModelName, setNewModelName] = useState('');
  const [newBaseUrl, setNewBaseUrl] = useState('');
  const [newApiKey, setNewApiKey] = useState('');
  const [newAlias, setNewAlias] = useState('');
  const [newKeyName, setNewKeyName] = useState('');
  const [copiedGateway, setCopiedGateway] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [pendingRevoke, setPendingRevoke] = useState<ApiKey | null>(null);
  const [revokingId, setRevokingId] = useState<string | null>(null);

  const gatewayUrl = 'http://localhost:8080/v1';

  const handleCopy = useCallback((text: string, type: 'gateway' | string) => {
    void navigator.clipboard.writeText(text);
    if (type === 'gateway') {
      setCopiedGateway(true);
      setTimeout(() => setCopiedGateway(false), 2000);
    } else {
      setCopiedKey(type);
      setTimeout(() => setCopiedKey(null), 2000);
    }
  }, []);

  const handleRevokeClick = useCallback((key: ApiKey) => {
    setPendingRevoke(key);
  }, []);

  const handleRevokeConfirm = useCallback(async () => {
    if (!pendingRevoke) return;

    const { id, name } = pendingRevoke;
    setRevokingId(id);

    try {
      const res = await fetch(
        `${CONFIG_BASE}/${tenantId}/api-keys/${id}`,
        {
          method: 'DELETE',
          credentials: 'include',
          headers: { Accept: 'application/json', 'x-csrf-token': '1' },
        }
      );

      if (res.status !== 204 && !res.ok) {
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

      await mutateKeys(
        (prev) => prev?.filter((k) => k.id !== id) ?? [],
        { revalidate: false },
      );

      push(`"${name}" revoked and invalidated successfully.`, 'success');
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Unknown error';
      push(`Failed to revoke "${name}": ${msg}`, 'error');
    } finally {
      setRevokingId(null);
      setPendingRevoke(null);
    }
  }, [pendingRevoke, tenantId, mutateKeys, push]);

  const handleAddProvider = useCallback(async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newApiKey.trim() || !newModelName.trim() || !newAlias.trim()) return;
    setSubmitting(true);
    try {
      // ── Phase 1: Auth — store the secret (only provider_name, api_key, key_alias)
      // model_name and base_url are intentionally NOT sent to Auth.
      const authRes = await fetch(`${AUTH_BASE}/provider-keys`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' },
        body: JSON.stringify({
          provider_name: newProviderName,
          api_key:       newApiKey,
          key_alias:     newAlias,
        }),
      });

      if (!authRes.ok) {
        const errBody: unknown = await authRes.json().catch(() => ({}));
        const msg =
          errBody !== null && typeof errBody === 'object' &&
          'error' in errBody && errBody.error !== null &&
          typeof errBody.error === 'object' && 'message' in errBody.error &&
          typeof (errBody.error as Record<string, unknown>).message === 'string'
            ? (errBody.error as { message: string }).message
            : `Auth service rejected the key (HTTP ${authRes.status})`;
        throw new Error(msg);
      }

      // ── Phase 2: Config — register the routing mesh entry
      // Auth succeeded; now tell Config about model_name, base_url, and schema_format.
      const configRes = await fetch(`${CONFIG_BASE}/${tenantId}/routing-mesh`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', 'x-csrf-token': '1' },
        body: JSON.stringify({
          model_name:    newModelName,
          base_url:      newBaseUrl,
          schema_format: newProviderName,   // schema_format is inferred from the provider
          key_alias:     newAlias,
          priority:      1,
          weight:        100,
        }),
      });

      if (!configRes.ok) {
        const errBody: unknown = await configRes.json().catch(() => ({}));
        const msg =
          errBody !== null && typeof errBody === 'object' &&
          'error' in errBody && errBody.error !== null &&
          typeof errBody.error === 'object' && 'message' in errBody.error &&
          typeof (errBody.error as Record<string, unknown>).message === 'string'
            ? (errBody.error as { message: string }).message
            : `Config service failed to register the routing mesh (HTTP ${configRes.status})`;
        throw new Error(msg);
      }

      // ── Success: reset form and re-fetch provider list
      await mutateProviders();
      setIsProviderModalOpen(false);
      setNewProviderName('openai');
      setNewModelName('');
      setNewBaseUrl('');
      setNewApiKey('');
      setNewAlias('');
      push(`Key "${newAlias}" secured and routing mesh updated for ${newModelName}.`, 'success');
    } catch (err) {
      push(err instanceof Error ? err.message : 'Failed to add provider key', 'error');
    } finally {
      setSubmitting(false);
    }
  }, [newProviderName, newModelName, newBaseUrl, newApiKey, newAlias, tenantId, mutateProviders, push]);


  const handleGenerate = useCallback((e: React.FormEvent) => {
    e.preventDefault();
    if (!newKeyName.trim()) return;
    push('Key generation coming soon — endpoint not yet provisioned.', 'info');
    setIsKeyModalOpen(false);
    setNewKeyName('');
  }, [newKeyName, push]);

  return (
    <>
      <motion.div
        variants={containerVariants}
        initial="hidden"
        animate="visible"
        className="grid grid-cols-12 gap-6 p-6 auto-rows-max text-[var(--on-surface)]"
      >
        {/* Breadcrumb (col-span-12) */}
        <motion.div variants={cardVariants} className="col-span-12 flex items-center gap-3 text-sm font-mono text-[var(--text-muted)] mb-2">
          <Link to="/dashboard" className="hover:text-[var(--on-surface)] transition-colors flex items-center gap-2 font-geist tracking-wide">
            <Shield className="w-4 h-4" />
            Dashboard
          </Link>
          <ArrowRight className="w-4 h-4" />
          <span className="text-[var(--on-surface)] font-geist">API Keys</span>
        </motion.div>

        {/* Page header (col-span-12) */}
        <motion.div variants={cardVariants} className="col-span-12 flex flex-col md:flex-row md:items-end justify-between gap-6 mb-8">
          <div>
            <h1 className="text-5xl font-extrabold tracking-tight mb-4 text-[var(--on-surface)] font-geist">
              Identity Vault
            </h1>
            <p className="text-sm text-[var(--text-muted)] max-w-2xl font-geist">
              Manage your LLM provider keys and issue virtual API access via a unified spatial interface.
            </p>
          </div>

          <div className="flex items-center gap-3 p-1 rounded-full bg-[rgba(255,255,255,0.02)]">
            <button
              onClick={() => push('Switch to Individual Vault to view personal keys.', 'info')}
              className={`relative px-6 py-2.5 rounded-full font-geist text-[10px] font-bold uppercase tracking-widest transition-all duration-300 ${!isTeam
                ? 'text-[var(--on-surface)] bg-[var(--surface-bright)] shadow-md'
                : 'text-[var(--text-muted)] hover:text-[var(--on-surface)]'
                }`}
            >
              {!isTeam && (
                <motion.div
                  layoutId="active-tab"
                  className="absolute inset-0 rounded-full"
                  initial={false}
                  transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                  style={{ boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.06)' }}
                />
              )}
              Individual
            </button>
            <button
              onClick={() => push('Switch to Team Vault to manage team keys.', 'info')}
              className={`relative px-6 py-2.5 rounded-full font-geist text-[10px] font-bold uppercase tracking-widest transition-all duration-300 ${isTeam
                ? 'text-[var(--on-surface)] bg-[var(--surface-bright)] shadow-md'
                : 'text-[var(--text-muted)] hover:text-[var(--on-surface)]'
                }`}
            >
              {isTeam && (
                <motion.div
                  layoutId="active-tab"
                  className="absolute inset-0 rounded-full"
                  initial={false}
                  transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                  style={{ boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.06)' }}
                />
              )}
              Team
            </button>
          </div>
        </motion.div>

        {/* ── LLM Provider Vault ──────────────────────────────────────────── */}
        <motion.div variants={cardVariants} className="col-span-12 mb-8">
          <div className="flex items-center justify-between mb-6">
            <div className="flex items-center gap-4">
              <div className="p-3 rounded-2xl bg-[rgba(34,211,238,0.1)]">
                <Server className="w-6 h-6 text-[var(--accent-cyan)]" />
              </div>
              <div>
                <h2 className="text-2xl font-bold text-[var(--on-surface)] tracking-tight font-geist">Provider Vault</h2>
                <p className={`${DATA_CLASS} text-[10px] mt-1`} style={{ color: 'var(--on-surface-muted)' }}>Securely store upstream LLM API keys.</p>
              </div>
            </div>
            <button
              onClick={() => setIsProviderModalOpen(true)}
              className={`${MECHANICAL_BTN} px-4 py-2 rounded-xl text-[10px] gap-2`}
            >
              <Plus className="w-4 h-4" />
              Forge New Key
            </button>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            {providerLoading ? (
              <div className="col-span-full flex items-center justify-center py-12 bg-[var(--surface-container-low)] rounded-2xl">
                <Loader2 className="w-6 h-6 animate-spin text-[var(--accent-cyan)] mr-3" />
                <span className={`${DATA_CLASS} text-[var(--text-muted)] text-[10px] uppercase tracking-widest`}>Loading vault...</span>
              </div>
            ) : providerError ? (
              <div className="col-span-full flex items-center gap-3 p-6 rounded-2xl bg-[rgba(244,63,94,0.1)]">
                <AlertTriangle className="w-5 h-5 text-[var(--primary-rose)]" />
                <p className="text-[var(--primary-rose)] font-geist text-sm">Failed to fetch provider keys from vault.</p>
              </div>
            ) : !providerKeys || providerKeys.length === 0 ? (
              <div className={`col-span-full py-16 ${INDUSTRIAL_SLOT}`}>
                <div className="text-5xl mb-6 opacity-30 grayscale">🔐</div>
                <h3 className={`text-xl font-bold mb-2 font-geist ${ENGRAVED_TEXT}`}>Vault is Empty</h3>
                <p className="text-[var(--text-muted)] mb-8 font-geist text-sm">Forge your first LLM provider key to start routing traffic.</p>
                <button
                  onClick={() => setIsProviderModalOpen(true)}
                  className={`${MECHANICAL_BTN} px-6 py-3 rounded-xl text-xs gap-3 font-black tracking-widest bg-[var(--accent-cyan)] !text-black`}
                >
                  <Plus className="w-5 h-5" />
                  INITIALIZE FORGE
                </button>
              </div>
            ) : (
              providerKeys.map((pk) => (
                <div key={pk.id} className={`${TACTILE_CARD} flex flex-col justify-between h-44 p-6 rounded-3xl`}>
                  <div className="flex items-start justify-between">
                    <span className={`text-lg font-bold capitalize tracking-tight font-geist ${ENGRAVED_TEXT}`}>
                      {pk.provider_name}
                    </span>
                    <StatusIndicator isActive={true} quotaUsed={0} />
                  </div>
                  <div className="mt-4 p-3 rounded-xl bg-black/20 shadow-[inset_0_2px_4px_rgba(0,0,0,0.3)] border border-white/5">
                    <p className={`${LABEL_CLASS} text-[9px] mb-1 opacity-60`}>Authenticated</p>
                    <p className={`${DATA_CLASS} text-xs font-bold`}>{new Date(pk.created_at).toLocaleDateString()}</p>
                  </div>
                </div>
              ))
            )}
          </div>
        </motion.div>

        {/* ── Virtual API Keys — Team Only ────────────────────────────────── */}
        {isTeam && (
          <motion.div variants={cardVariants} className="col-span-12">
            <div className="flex items-center justify-between mb-6">
              <div className="flex items-center gap-4">
                <div className="p-3 rounded-2xl bg-[rgba(34,211,238,0.1)]">
                  <Key className="w-6 h-6 text-[var(--accent-cyan)]" />
                </div>
                <div>
                  <h2 className="text-2xl font-bold text-[var(--on-surface)] tracking-tight font-geist">Virtual API Keys</h2>
                  <p className={`${DATA_CLASS} text-[10px] mt-1`} style={{ color: 'var(--on-surface-muted)' }}>Manage access tokens for your applications and team.</p>
                </div>
              </div>
              <button
                onClick={() => setIsKeyModalOpen(true)}
                className={`${MECHANICAL_BTN} px-5 py-2 rounded-xl text-[10px] gap-2 bg-gradient-to-br from-[var(--primary-amber)] to-[var(--primary-rose)] !text-black !border-b-[4px]`}
              >
                <Plus className="w-4 h-4" />
                MINT VIRTUAL KEY
              </button>
            </div>

            <div className="flex flex-col mb-8">
              {keysLoading ? (
                <div className="flex items-center justify-center py-12 bg-[var(--surface-container-low)] rounded-2xl">
                  <Loader2 className="w-6 h-6 animate-spin text-[var(--accent-cyan)] mr-3" />
                  <span className={`${DATA_CLASS} text-[var(--text-muted)] text-[10px] uppercase tracking-widest`}>Loading virtual keys...</span>
                </div>
              ) : keysError ? (
                <div className="flex items-center gap-3 p-6 rounded-2xl bg-[rgba(244,63,94,0.1)]">
                  <AlertTriangle className="w-5 h-5 text-[var(--primary-rose)]" />
                  <p className="text-[var(--primary-rose)] font-geist text-sm">Failed to fetch virtual API keys.</p>
                </div>
              ) : !keys || keys.length === 0 ? (
                <div className={`py-20 ${INDUSTRIAL_SLOT} border-none`}>
                  <div className="text-5xl mb-6 opacity-30 grayscale">🔑</div>
                  <h3 className={`text-xl font-bold mb-2 font-geist ${ENGRAVED_TEXT}`}>No Virtual Tokens</h3>
                  <p className="text-[var(--text-muted)] mb-8 font-geist text-sm">Forge a virtual key to authenticate your distributed applications.</p>
                  <button
                    onClick={() => setIsKeyModalOpen(true)}
                    className={`${MECHANICAL_BTN} px-6 py-3 rounded-xl text-xs gap-3 font-black tracking-widest bg-[var(--accent-cyan)] !text-black`}
                  >
                    <Plus className="w-5 h-5" />
                    GENERATE FIRST TOKEN
                  </button>
                </div>
              ) : (
                <div className="flex flex-col gap-8">
                  {Object.entries(
                    keys.reduce((acc, key) => {
                      const group = key.model_name || 'Global Keys';
                      if (!acc[group]) acc[group] = [];
                      acc[group].push(key);
                      return acc;
                    }, {} as Record<string, ApiKey[]>)
                  ).map(([groupName, groupKeys]) => (
                    <div key={groupName} className="flex flex-col">
                      <div className="flex items-center gap-3 mb-4 pl-2">
                        <div className="w-2 h-2 rounded-full bg-[var(--accent-cyan)] shadow-[0_0_8px_rgba(34,211,238,0.6)]" />
                        <h3 className={`text-sm font-bold tracking-widest uppercase font-geist ${ENGRAVED_TEXT}`}>
                          {groupName}
                        </h3>
                      </div>
                      <div className="flex flex-col rounded-3xl overflow-hidden bg-[var(--surface-lowest)] shadow-[0_12px_32px_-8px_rgba(0,0,0,0.5),inset_0_1px_1px_rgba(255,255,255,0.05)] border-t border-[rgba(255,255,255,0.05)]">
                        {groupKeys.map((keyItem, index) => {
                          const isRevokingThis = revokingId === keyItem.id;
                          const keyString = `sk_live_${keyItem.id.replace(/-/g, '')}`;
                          const zebraBg = index % 2 === 0 ? 'bg-[var(--surface-lowest)]' : 'bg-[var(--surface-container-low)]';
                          
                          return (
                            <div
                              key={keyItem.id}
                              className={`flex flex-col xl:flex-row xl:items-center justify-between p-6 transition-colors hover:bg-[var(--surface-container)] ${zebraBg}`}
                            >
                              <div className="flex items-center gap-6 mb-6 xl:mb-0 w-full xl:w-1/3">
                                <StatusIndicator isActive={keyItem.is_active} quotaUsed={keyItem.quota_used ?? 0} />
                                <div>
                                  <h4 className={`text-lg font-bold tracking-tight mb-1 font-geist ${ENGRAVED_TEXT}`}>{keyItem.name}</h4>
                                  <div
                                    className="group relative cursor-pointer inline-block overflow-hidden rounded-md"
                                    onClick={() => handleCopy(keyString, 'key')}
                                    title="Click to copy"
                                  >
                                    <div className="absolute inset-0 bg-white/5 opacity-0 group-hover:opacity-100 transition-opacity z-10" />
                                    <span className={`${DATA_CLASS} text-xs transition-all duration-500 block px-2 py-1 bg-black/40 shadow-[inset_0_1px_3px_rgba(0,0,0,0.5)] rounded tracking-tighter ${copiedKey === 'key' ? 'text-emerald-400' : 'text-[var(--text-muted)]'}`}>
                                      <span className="blur-md group-hover:blur-none transition-all duration-300">
                                        {keyString}
                                      </span>
                                    </span>
                                  </div>
                                </div>
                              </div>

                              <div className="flex flex-col sm:flex-row items-start sm:items-center gap-8 w-full xl:w-2/3 justify-end">
                                <div className="flex flex-col items-center min-w-[100px]">
                                  <p className={`${LABEL_CLASS} mb-2`}>24H Trend</p>
                                  <InlineSparkline data={keyItem.usage_trend_24h ?? []} />
                                </div>
                                <div className="flex flex-col items-center min-w-[120px]">
                                  <p className={`${LABEL_CLASS} mb-2`}>Activity Matrix</p>
                                  <KeyUsageHeatmap dailyRequests={keyItem.daily_requests ?? []} />
                                </div>
                                <div className="flex flex-col items-start sm:items-end min-w-[100px]">
                                  <p className={`${LABEL_CLASS} mb-1`}>Created</p>
                                  <span className={`${DATA_CLASS} text-[11px] text-[var(--text-muted)]`}>{new Date(keyItem.created_at).toLocaleDateString()}</span>
                                  {keyItem.expires_at && (
                                    <>
                                      <p className={`${LABEL_CLASS} mt-2 mb-1`}>Expires</p>
                                      <span className={`${DATA_CLASS} text-[11px] text-[var(--primary-amber)]`}>{new Date(keyItem.expires_at).toLocaleDateString()}</span>
                                    </>
                                  )}
                                </div>
                                <div className="flex items-center gap-4 sm:pl-4 border-l border-[rgba(255,255,255,0.02)]">
                                  <QuotaRadialBar quotaUsed={keyItem.quota_used ?? 0} />
                                  {keyItem.is_active && (
                                    <button
                                      onClick={() => handleRevokeClick(keyItem)}
                                      className={`${MECHANICAL_BTN} p-2.5 rounded-xl text-rose-400/80 hover:text-rose-400 hover:bg-[rgba(244,63,94,0.1)]`}
                                      title="Revoke Key"
                                    >
                                      {isRevokingThis ? <Loader2 className="w-4 h-4 animate-spin" /> : <Trash2 className="w-4 h-4" />}
                                    </button>
                                  )}
                                </div>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Gateway URL strip */}
            <div className={`${TACTILE_CARD} flex flex-col sm:flex-row items-start sm:items-center justify-between gap-6 p-7 rounded-[2rem]`}>
              <div className="flex items-center gap-4">
                <div className="p-4 rounded-2xl bg-black/30 shadow-[inset_0_2px_8px_rgba(0,0,0,0.5)]">
                  <Globe className="w-6 h-6 text-[var(--accent-cyan)]" />
                </div>
                <div>
                  <h3 className={`text-xl font-bold tracking-tight font-geist ${ENGRAVED_TEXT}`}>Gateway Endpoint</h3>
                  <code className={`${DATA_CLASS} text-[10px] mt-1 block opacity-60 tracking-wider`}>{gatewayUrl}</code>
                </div>
              </div>
              <button
                onClick={() => handleCopy(gatewayUrl, 'gateway')}
                className={`${MECHANICAL_BTN} px-6 py-3 rounded-xl text-xs gap-3 font-black tracking-widest`}
              >
                {copiedGateway ? <Check className="w-4 h-4 text-emerald-400" /> : <Copy className="w-4 h-4" />}
                {copiedGateway ? 'LINK COPIED' : 'COPY ENDPOINT'}
              </button>
            </div>
          </motion.div>
        )}

        {/* ── Individual account — Gateway Key Card ────────────────────────── */}
        {!isTeam && (
          <motion.div variants={cardVariants} className="col-span-12 lg:col-span-8 lg:col-start-3">
            <div className={`${TACTILE_CARD} p-8 rounded-[2.5rem]`}>
              <div className="flex items-center gap-4 mb-8">
                <div className="p-4 rounded-2xl bg-black/30 shadow-[inset_0_2px_8px_rgba(0,0,0,0.5)]">
                  <Lock className="w-6 h-6 text-[var(--accent-cyan)]" />
                </div>
                <div>
                  <h2 className={`text-3xl font-bold tracking-tight font-geist ${ENGRAVED_TEXT}`}>Master Access Key</h2>
                  <p className={`${DATA_CLASS} text-[10px] mt-1 uppercase tracking-widest opacity-40`}>Personal cryptographic token</p>
                </div>
              </div>

              <div className="bg-black/30 shadow-[inset_0_4px_12px_rgba(0,0,0,0.6)] rounded-2xl p-6 mb-8 flex items-center justify-between gap-4 border border-white/5">
                <code className={`${DATA_CLASS} text-[var(--accent-cyan)] text-base break-all group relative cursor-pointer px-3 py-2 rounded-lg overflow-hidden`} onClick={() => user?.redeyeApiKey && handleCopy(user.redeyeApiKey, 'user-key')} title="Click to copy">
                  <span className="blur-xl group-hover:blur-none transition-all duration-500 font-bold tracking-tighter">
                    {user?.redeyeApiKey ?? 'No key available'}
                  </span>
                </code>
                <button
                  onClick={() => user?.redeyeApiKey && handleCopy(user.redeyeApiKey, 'user-key')}
                  disabled={!user?.redeyeApiKey}
                  className={`${MECHANICAL_BTN} p-4 rounded-xl !bg-[var(--surface-container)]`}
                >
                  {copiedKey === 'user-key' ? <Check className="w-5 h-5 text-emerald-400" /> : <Copy className="w-5 h-5" />}
                </button>
              </div>

              <div className="bg-[var(--surface-container-low)] rounded-2xl p-5 mb-8 flex items-center justify-between gap-4">
                <div>
                  <div className="flex items-center gap-2 mb-2">
                    <Globe className="w-4 h-4 text-[var(--accent-cyan)]" />
                    <span className="text-sm font-bold text-[var(--on-surface)] font-geist">Gateway URL</span>
                  </div>
                  <code className={`${DATA_CLASS} text-[10px] text-[var(--on-surface-muted)]`}>{gatewayUrl}</code>
                </div>
                <motion.button
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                  onClick={() => handleCopy(gatewayUrl, 'gateway')}
                  className="p-3 rounded-xl bg-[var(--surface-bright)] hover:bg-[rgba(255,255,255,0.1)] transition-all text-[var(--on-surface)]"
                >
                  {copiedGateway ? <Check className="w-5 h-5 text-[var(--accent-cyan)]" /> : <Copy className="w-5 h-5" />}
                </motion.button>
              </div>

              <div className={`p-10 ${INDUSTRIAL_SLOT} border-none`}>
                <h3 className={`text-2xl font-bold mb-3 font-geist ${ENGRAVED_TEXT}`}>Ready to Scale?</h3>
                <p className="text-[var(--text-muted)] mb-8 max-w-md mx-auto font-geist text-sm leading-relaxed">
                  Upgrade to a Team Plan to issue multiple virtual keys, track detailed usage per-key, and manage quotas.
                </p>
                <button
                  onClick={() => push('Upgrade flow coming soon.', 'info')}
                  className={`${MECHANICAL_BTN} px-8 py-4 rounded-2xl text-xs gap-3 font-black tracking-widest bg-gradient-to-br from-[var(--primary-amber)] to-[var(--primary-rose)] !text-black !border-b-[5px]`}
                >
                  <Users className="w-5 h-5" />
                  UPGRADE INFRASTRUCTURE
                </button>
              </div>
            </div>
          </motion.div>
        )}
      </motion.div>

      {/* ── Modals ────────────────────────────────────────────────────── */}
      <AnimatePresence>
        {isProviderModalOpen && (
          <motion.div
            variants={modalVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-[var(--bg-canvas)]/80 backdrop-blur-md"
          >
            <motion.div
              variants={modalVariants}
              className="bg-[var(--surface-container)] shadow-2xl shadow-[var(--accent-cyan)]/10 rounded-3xl w-full max-w-md overflow-hidden"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-center justify-between px-6 py-5 bg-[rgba(255,255,255,0.02)]">
                <h3 className="text-lg font-bold text-[var(--on-surface)] flex items-center gap-2 font-geist">
                  <ShieldCheck className="w-5 h-5 text-[var(--accent-cyan)]" />
                  Add Provider Key
                </h3>
                <button
                  onClick={() => setIsProviderModalOpen(false)}
                  className="text-[var(--text-muted)] hover:text-[var(--on-surface)] transition-colors p-1"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <form onSubmit={handleAddProvider} className="p-6 space-y-5">
                <div className="bg-[rgba(34,211,238,0.1)] p-4 rounded-2xl">
                  <div className="flex items-start gap-3">
                    <AlertTriangle className="w-4 h-4 text-[var(--accent-cyan)] shrink-0 mt-0.5" />
                    <p className="text-sm text-[var(--on-surface)] font-geist">
                      Your API key is AES-256-GCM encrypted. We never log plaintext keys.
                    </p>
                  </div>
                </div>

                <div className="space-y-4">
                  <div>
                    <label htmlFor="providerName" className={`${LABEL_CLASS} mb-2 block`}>
                      Provider Name
                    </label>
                    <select
                      id="providerName"
                      name="providerName"
                      required
                      value={newProviderName}
                      onChange={(e) => setNewProviderName(e.target.value)}
                      className="w-full rounded-xl bg-[#080808] shadow-[inset_0_2px_8px_rgba(0,0,0,0.8),inset_0_1px_0_rgba(255,255,255,0.02)] px-4 py-3 text-sm text-[var(--on-surface)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-cyan)] transition-all"
                    >
                      <option value="openai" className="bg-[var(--surface-container)]">OpenAI</option>
                      <option value="google" className="bg-[var(--surface-container)]">Google</option>
                      <option value="anthropic" className="bg-[var(--surface-container)]">Anthropic</option>
                      <option value="deepseek" className="bg-[var(--surface-container)]">DeepSeek</option>
                      <option value="custom" className="bg-[var(--surface-container)]">Custom</option>
                    </select>
                  </div>

                  <div>
                    <label htmlFor="modelName" className={`${LABEL_CLASS} mb-2 block`}>
                      Model Name
                    </label>
                    <input
                      id="modelName"
                      name="modelName"
                      type="text"
                      required
                      placeholder="e.g., gemini-2.0-flash"
                      value={newModelName}
                      onChange={(e) => setNewModelName(e.target.value)}
                      className="w-full rounded-xl bg-[#080808] shadow-[inset_0_2px_8px_rgba(0,0,0,0.8),inset_0_1px_0_rgba(255,255,255,0.02)] px-4 py-3 text-sm text-[var(--on-surface)] placeholder:text-[var(--text-subtle)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-cyan)] transition-all"
                    />
                  </div>

                  <div>
                    <label htmlFor="baseUrl" className={`${LABEL_CLASS} mb-2 block`}>
                      Base URL
                    </label>
                    <input
                      id="baseUrl"
                      name="baseUrl"
                      type="text"
                      required
                      placeholder="https://api.openai.com/v1"
                      value={newBaseUrl}
                      onChange={(e) => setNewBaseUrl(e.target.value)}
                      className="w-full rounded-xl bg-[#080808] shadow-[inset_0_2px_8px_rgba(0,0,0,0.8),inset_0_1px_0_rgba(255,255,255,0.02)] px-4 py-3 text-sm text-[var(--on-surface)] placeholder:text-[var(--text-subtle)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-cyan)] transition-all"
                    />
                  </div>

                  <div>
                    <label htmlFor="apiKey" className={`${LABEL_CLASS} mb-2 block`}>
                      API Key
                    </label>
                    <input
                      id="apiKey"
                      name="apiKey"
                      type="password"
                      required
                      value={newApiKey}
                      onChange={(e) => setNewApiKey(e.target.value)}
                      className="w-full rounded-xl bg-[#080808] shadow-[inset_0_2px_8px_rgba(0,0,0,0.8),inset_0_1px_0_rgba(255,255,255,0.02)] px-4 py-3 text-sm text-[var(--on-surface)] font-mono placeholder:text-[var(--text-subtle)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-cyan)] transition-all"
                    />
                  </div>

                  <div>
                    <label htmlFor="keyAlias" className={`${LABEL_CLASS} mb-2 block`}>
                      Key Alias
                    </label>
                    <input
                      id="keyAlias"
                      name="keyAlias"
                      type="text"
                      required
                      placeholder="e.g., Primary-Key or Grok-1"
                      value={newAlias}
                      onChange={(e) => setNewAlias(e.target.value)}
                      className="w-full rounded-xl bg-[#080808] shadow-[inset_0_2px_8px_rgba(0,0,0,0.8),inset_0_1px_0_rgba(255,255,255,0.02)] px-4 py-3 text-sm text-[var(--on-surface)] placeholder:text-[var(--text-subtle)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-cyan)] transition-all"
                    />
                  </div>
                </div>

                <div className="flex gap-3 justify-end pt-4">
                  <button
                    type="button"
                    onClick={() => setIsProviderModalOpen(false)}
                    className="rounded-xl px-5 py-2.5 text-sm font-semibold text-[var(--text-muted)] hover:text-[var(--on-surface)] hover:bg-[rgba(255,255,255,0.05)] transition-colors font-geist"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={submitting}
                    className={`${MECHANICAL_BTN} px-6 py-3 rounded-xl text-sm bg-[var(--accent-cyan)] !text-black font-black !border-b-[4px]`}
                  >
                    {submitting ? <><Loader2 className="w-4 h-4 animate-spin mr-2" />Syncing…</> : 'FORGE KEY'}
                  </button>
                </div>
              </form>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {isKeyModalOpen && isTeam && (
          <motion.div
            variants={modalVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-[var(--bg-canvas)]/80 backdrop-blur-md"
          >
            <motion.div
              variants={modalVariants}
              className="bg-[var(--surface-container)] shadow-2xl shadow-[var(--primary-rose)]/10 rounded-3xl w-full max-w-md overflow-hidden"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-center justify-between px-6 py-5 bg-[rgba(255,255,255,0.02)]">
                <h3 className="text-lg font-bold text-[var(--on-surface)] flex items-center gap-2 font-geist">
                  <ShieldCheck className="w-5 h-5 text-[var(--primary-rose)]" />
                  Generate Virtual Key
                </h3>
                <button
                  onClick={() => setIsKeyModalOpen(false)}
                  className="text-[var(--text-muted)] hover:text-[var(--on-surface)] transition-colors p-1"
                >
                  <X className="w-5 h-5" />
                </button>
              </div>

              <form onSubmit={handleGenerate} className="p-6">
                <div className="bg-[rgba(244,63,94,0.1)] p-4 rounded-2xl mb-6">
                  <div className="flex items-start gap-3">
                    <AlertTriangle className="w-4 h-4 text-[var(--primary-rose)] shrink-0 mt-0.5" />
                    <p className="text-sm text-[var(--on-surface)] font-geist">
                      For security, the new key is shown only once. Be ready to copy it.
                    </p>
                  </div>
                </div>

                <div className="space-y-4 mb-8">
                  <div>
                    <label htmlFor="new-key-name" className={`${LABEL_CLASS} mb-2 block`}>
                      Key Name
                    </label>
                    <input
                      id="new-key-name"
                      type="text"
                      required
                      autoFocus
                      placeholder="e.g. Production Frontend App"
                      value={newKeyName}
                      onChange={(e) => setNewKeyName(e.target.value)}
                      className="w-full rounded-xl bg-[#080808] shadow-[inset_0_2px_8px_rgba(0,0,0,0.8),inset_0_1px_0_rgba(255,255,255,0.02)] px-4 py-3 text-sm text-[var(--on-surface)] placeholder:text-[var(--text-subtle)] focus:outline-none focus:ring-1 focus:ring-[var(--primary-rose)] transition-all"
                    />
                  </div>
                </div>

                <div className="flex gap-3 justify-end">
                  <button
                    type="button"
                    onClick={() => setIsKeyModalOpen(false)}
                    className="rounded-xl px-5 py-2.5 text-sm font-semibold text-[var(--text-muted)] hover:text-[var(--on-surface)] hover:bg-[rgba(255,255,255,0.05)] transition-colors font-geist"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    className={`${MECHANICAL_BTN} px-6 py-3 rounded-xl text-sm bg-gradient-to-br from-[var(--primary-amber)] to-[var(--primary-rose)] !text-black font-black !border-b-[4px]`}
                  >
                    MINT NEW TOKEN
                  </button>
                </div>
              </form>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence>
        {pendingRevoke && (
          <RevokeConfirmModal
            keyToRevoke={pendingRevoke}
            isRevoking={revokingId === pendingRevoke.id}
            onConfirm={() => void handleRevokeConfirm()}
            onCancel={() => setPendingRevoke(null)}
          />
        )}
      </AnimatePresence>

      <ToastList toasts={toasts} dismiss={dismiss} />
    </>
  );
}
