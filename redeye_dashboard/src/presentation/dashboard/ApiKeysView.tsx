// Presentation Dashboard — ApiKeysView
// 2026 UX Design Principles: Card-based layout, fluid motion, progressive disclosure
// Split view: LLM Provider Vault (always visible) + Virtual API Keys (team only)
// Theme: "Cool Revival / Neon Crimson" — Dark Red/Glass aesthetic

import { useState } from 'react';
import { Key, Plus, Trash2, X, AlertTriangle, ShieldCheck, Copy, Check, Globe, Loader2, Lock, Server, User, Users, ArrowRight, Sparkles, Shield } from 'lucide-react';
import useSWR from 'swr';
import { useAuth } from '../context/AuthContext';
import { motion, AnimatePresence } from 'framer-motion';
import { Link } from 'react-router-dom';
import { SUPPORTED_PROVIDERS } from '../../data/constants/providers';

export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  created_at: string;
  status: string;
}

interface ProviderKey {
  id: string;
  provider_name: string;
  created_at: string;
}

const fetcher = async (url: string) => {
  const res = await fetch(url, { 
    credentials: 'include',
    headers: { 'Content-Type': 'application/json' }
  });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

// 2026 UX: Motion variants for smooth transitions
const cardVariants = {
  hidden: { opacity: 0, y: 20, scale: 0.95 },
  visible: { opacity: 1, y: 0, scale: 1, transition: { duration: 0.4, ease: [0.25, 0.1, 0.25, 1] as const } },
  hover: { scale: 1.02, transition: { duration: 0.2 } }
};

const containerVariants = {
  hidden: { opacity: 0 },
  visible: { opacity: 1, transition: { staggerChildren: 0.1, delayChildren: 0.1 } }
};

const modalVariants = {
  hidden: { opacity: 0, scale: 0.9 },
  visible: { opacity: 1, scale: 1, transition: { duration: 0.3, ease: [0.25, 0.1, 0.25, 1] as const } },
  exit: { opacity: 0, scale: 0.9, transition: { duration: 0.2 } }
};

export function ApiKeysView() {
  const { user } = useAuth();
  const accountType = user?.accountType ?? 'individual';
  const isTeam = accountType === 'team';

  // Virtual API Keys (for team view)
  const { data: keys, error: keysError, isLoading: keysLoading } = useSWR<ApiKey[]>(
    isTeam ? 'http://localhost:8084/v1/auth/api-keys' : null,
    fetcher
  );

  // Provider Keys (for LLM Provider Vault)
  const { data: providerKeys, error: providerError, isLoading: providerLoading, mutate: mutateProviders } = useSWR<ProviderKey[]>(
    'http://localhost:8084/v1/auth/provider-keys',
    fetcher
  );

  // Modal states
  const [isProviderModalOpen, setIsProviderModalOpen] = useState(false);
  const [isKeyModalOpen, setIsKeyModalOpen] = useState(false);
  const [newProviderName, setNewProviderName] = useState('openai');
  const [newProviderKey, setNewProviderKey] = useState('');
  const [newKeyName, setNewKeyName] = useState('');
  const [copiedGateway, setCopiedGateway] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const gatewayUrl = 'http://localhost:8080/v1';

  const handleCopy = (text: string, type: 'gateway' | string) => {
    navigator.clipboard.writeText(text);
    if (type === 'gateway') {
      setCopiedGateway(true);
      setTimeout(() => setCopiedGateway(false), 2000);
    } else {
      setCopiedKey(type);
      setTimeout(() => setCopiedKey(null), 2000);
    }
  };

  const handleRevoke = async (_id: string) => {
    alert('Revoke endpoint not yet implemented');
  };

  const handleAddProvider = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newProviderKey.trim()) return;
    setSubmitting(true);
    try {
      const res = await fetch('http://localhost:8084/v1/auth/provider-keys', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider_name: newProviderName,
          provider_api_key: newProviderKey,
        }),
      });
      if (!res.ok) throw new Error('Failed to add provider key');
      await mutateProviders();
      setIsProviderModalOpen(false);
      setNewProviderKey('');
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to add provider key');
    } finally {
      setSubmitting(false);
    }
  };

  const handleGenerate = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newKeyName.trim()) return;
    alert('Generate endpoint not yet implemented');
    setIsKeyModalOpen(false);
  };

  return (
    <motion.div 
      variants={containerVariants}
      initial="hidden"
      animate="visible"
      className="min-h-screen bg-gradient-to-br from-slate-50 via-white to-slate-100 dark:from-slate-950 dark:via-slate-900 dark:to-slate-950 p-6"
    >
      {/* 2026 UX: Navigation Breadcrumb */}
      <motion.div 
        variants={cardVariants}
        className="mb-8 flex items-center gap-3 text-sm"
      >
        <Link 
          to="/dashboard" 
          className="text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 transition-colors flex items-center gap-2"
        >
          <Shield className="w-4 h-4" />
          Dashboard
        </Link>
        <ArrowRight className="w-4 h-4 text-slate-400" />
        <span className="text-slate-900 dark:text-slate-100 font-medium">API Keys</span>
      </motion.div>

      {/* 2026 UX: Page Header with gradient */}
      <motion.div 
        variants={cardVariants}
        className="text-center mb-12"
      >
        <h1 className="text-4xl sm:text-5xl font-bold bg-gradient-to-r from-cyan-600 via-teal-500 to-emerald-400 dark:from-cyan-400 dark:via-teal-300 dark:to-emerald-200 bg-clip-text text-transparent mb-4">
          API Keys &amp; Providers
        </h1>
        <p className="text-lg text-slate-600 dark:text-slate-400 max-w-2xl mx-auto">
          Manage your LLM provider vault and virtual API keys with enterprise-grade security
        </p>
        <div className="flex items-center justify-center gap-2 mt-4">
          <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-gradient-to-r from-cyan-500/10 to-teal-500/10 dark:from-cyan-500/20 dark:to-teal-500/20 border border-cyan-200 dark:border-cyan-800">
            <Sparkles className="w-4 h-4 text-cyan-600 dark:text-cyan-400" />
            <span className="text-sm font-medium text-cyan-700 dark:text-cyan-300">
              {isTeam ? 'Team Account' : 'Individual Account'}
            </span>
          </div>
        </div>
      </motion.div>
      {/* 2026 UX: LLM Provider Vault Card */}
      <motion.div 
        variants={cardVariants}
        whileHover="hover"
        className="mb-8"
      >
        <div className="bg-white/90 dark:bg-slate-900/60 backdrop-blur-xl border border-slate-200/50 dark:border-slate-700/50 rounded-2xl shadow-xl dark:shadow-2xl shadow-cyan-500/10 dark:shadow-cyan-500/5 overflow-hidden">
          {/* Card Header */}
          <div className="bg-gradient-to-r from-cyan-500/5 to-teal-500/5 dark:from-cyan-500/10 dark:to-teal-500/10 px-6 py-4 border-b border-slate-200/50 dark:border-slate-700/50">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-xl bg-gradient-to-br from-cyan-500 to-teal-500 shadow-lg">
                  <Server className="w-5 h-5 text-white" />
                </div>
                <div>
                  <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100">LLM Provider Vault</h2>
                  <p className="text-sm text-slate-600 dark:text-slate-400">Secure encrypted key storage</p>
                </div>
              </div>
              <motion.button
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                onClick={() => setIsProviderModalOpen(true)}
                className="px-4 py-2 bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 text-white font-semibold rounded-xl shadow-lg flex items-center gap-2 transition-all duration-200"
              >
                <Plus className="w-4 h-4" />
                Add Provider
              </motion.button>
            </div>
          </div>

          {/* Card Content */}
          <div className="p-6">
            {/* Provider Keys Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {providerLoading ? (
                <div className="col-span-full flex items-center justify-center py-12">
                  <Loader2 className="w-6 h-6 animate-spin text-cyan-500 mr-3" />
                  <span className="text-slate-600 dark:text-slate-400">Loading provider keys...</span>
                </div>
              ) : providerError ? (
                <div className="col-span-full text-center py-12">
                  <div className="text-rose-500 mb-2">⚠️</div>
                  <p className="text-rose-600 dark:text-rose-400">Failed to fetch provider keys</p>
                </div>
              ) : !providerKeys || providerKeys.length === 0 ? (
                <motion.div 
                  variants={cardVariants}
                  className="col-span-full text-center py-12 border-2 border-dashed border-slate-300 dark:border-slate-600 rounded-xl"
                >
                  <div className="text-4xl mb-4">🔐</div>
                  <h3 className="text-lg font-semibold text-slate-700 dark:text-slate-300 mb-2">No Provider Keys</h3>
                  <p className="text-slate-600 dark:text-slate-400 mb-4">Add your first LLM provider key to get started</p>
                  <motion.button
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => setIsProviderModalOpen(true)}
                    className="px-6 py-3 bg-gradient-to-r from-cyan-500 to-teal-500 text-white font-semibold rounded-xl shadow-lg inline-flex items-center gap-2"
                  >
                    <Plus className="w-4 h-4" />
                    Add Your First Provider
                  </motion.button>
                </motion.div>
              ) : (
                providerKeys.map((pk, index) => (
                  <motion.div
                    key={pk.id}
                    variants={cardVariants}
                    custom={index}
                    whileHover="hover"
                    className="bg-slate-50 dark:bg-slate-800/50 border border-slate-200 dark:border-slate-700 rounded-xl p-4 hover:shadow-lg transition-all duration-300"
                  >
                    <div className="flex items-center justify-between mb-3">
                      <span className="text-sm font-semibold text-slate-900 dark:text-slate-100 capitalize">
                        {pk.provider_name}
                      </span>
                      <div className="flex items-center gap-1.5">
                        <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                        <span className="text-xs text-emerald-600 dark:text-emerald-400 font-medium">Active</span>
                      </div>
                    </div>
                    <div className="text-xs text-slate-500 dark:text-slate-400">
                      Added {new Date(pk.created_at).toLocaleDateString()}
                    </div>
                  </motion.div>
                ))
              )}
            </div>
          </div>
        </div>
      </motion.div>

      {/* 2026 UX: Virtual API Keys (Team Only) */}
      {isTeam && (
        <motion.div 
          variants={cardVariants}
          whileHover="hover"
          className="mb-8"
        >
          <div className="bg-white/90 dark:bg-rose-950/20 backdrop-blur-xl border border-rose-200/50 dark:border-rose-800/50 rounded-2xl shadow-xl dark:shadow-2xl shadow-rose-500/10 dark:shadow-rose-500/5 overflow-hidden">
            {/* Card Header */}
            <div className="bg-gradient-to-r from-rose-500/5 to-pink-500/5 dark:from-rose-500/10 dark:to-pink-500/10 px-6 py-4 border-b border-rose-200/50 dark:border-rose-800/50">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-xl bg-gradient-to-br from-rose-500 to-pink-500 shadow-lg">
                    <Key className="w-5 h-5 text-white" />
                  </div>
                  <div>
                    <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100">Virtual API Keys</h2>
                    <p className="text-sm text-slate-600 dark:text-slate-400">Team key management</p>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="px-3 py-1.5 rounded-full bg-gradient-to-r from-rose-500/10 to-pink-500/10 border border-rose-200 dark:border-rose-800">
                    <span className="text-xs font-semibold text-rose-700 dark:text-rose-300 flex items-center gap-1.5">
                      <Users className="w-3 h-3" />
                      Team Plan
                    </span>
                  </div>
                  <motion.button
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => setIsKeyModalOpen(true)}
                    className="px-4 py-2 bg-gradient-to-r from-rose-500 to-pink-500 hover:from-rose-400 hover:to-pink-400 text-white font-semibold rounded-xl shadow-lg flex items-center gap-2 transition-all duration-200"
                  >
                    <Plus className="w-4 h-4" />
                    Generate Key
                  </motion.button>
                </div>
              </div>
            </div>

            {/* Card Content */}
            <div className="p-6">
              {/* Keys Grid */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {keysLoading ? (
                  <div className="col-span-full flex items-center justify-center py-12">
                    <Loader2 className="w-6 h-6 animate-spin text-rose-500 mr-3" />
                    <span className="text-slate-600 dark:text-slate-400">Loading API keys...</span>
                  </div>
                ) : keysError ? (
                  <div className="col-span-full text-center py-12">
                    <div className="text-rose-500 mb-2">⚠️</div>
                    <p className="text-rose-600 dark:text-rose-400">Failed to fetch API keys</p>
                  </div>
                ) : !keys || keys.length === 0 ? (
                  <motion.div 
                    variants={cardVariants}
                    className="col-span-full text-center py-12 border-2 border-dashed border-rose-300 dark:border-rose-600 rounded-xl"
                  >
                    <div className="text-4xl mb-4">🔑</div>
                    <h3 className="text-lg font-semibold text-slate-700 dark:text-slate-300 mb-2">No Virtual Keys</h3>
                    <p className="text-slate-600 dark:text-slate-400 mb-4">Generate your first virtual API key for your team</p>
                    <motion.button
                      whileHover={{ scale: 1.05 }}
                      whileTap={{ scale: 0.95 }}
                      onClick={() => setIsKeyModalOpen(true)}
                      className="px-6 py-3 bg-gradient-to-r from-rose-500 to-pink-500 text-white font-semibold rounded-xl shadow-lg inline-flex items-center gap-2"
                    >
                      <Plus className="w-4 h-4" />
                      Generate First Key
                    </motion.button>
                  </motion.div>
                ) : (
                  keys.map((keyItem, index) => (
                    <motion.div
                      key={keyItem.id}
                      variants={cardVariants}
                      custom={index}
                      whileHover="hover"
                      className="bg-slate-50 dark:bg-rose-950/30 border border-rose-200 dark:border-rose-800 rounded-xl p-4 hover:shadow-lg transition-all duration-300"
                    >
                      <div className="flex items-center justify-between mb-3">
                        <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                          {keyItem.name}
                        </span>
                        {keyItem.status === 'Active' && (
                          <motion.button
                            whileHover={{ scale: 1.05 }}
                            whileTap={{ scale: 0.95 }}
                            onClick={() => handleRevoke(keyItem.id)}
                            className="px-3 py-1.5 bg-rose-100 dark:bg-rose-500/20 text-rose-600 dark:text-rose-400 rounded-lg text-xs font-medium hover:bg-rose-200 dark:hover:bg-rose-500/30 transition-all duration-200 flex items-center gap-1"
                          >
                            <Trash2 className="w-3 h-3" />
                            Revoke
                          </motion.button>
                        )}
                      </div>
                      <div className="text-xs text-slate-500 dark:text-slate-400 mb-2">
                        {keyItem.key_hash.substring(0, 16)}...
                      </div>
                      <div className="flex items-center gap-1.5">
                        <div className={`w-2 h-2 rounded-full ${keyItem.status === 'Active' ? 'bg-emerald-500 animate-pulse' : 'bg-rose-500'}`} />
                        <span className={`text-xs font-medium ${keyItem.status === 'Active' ? 'text-emerald-600 dark:text-emerald-400' : 'text-rose-600 dark:text-rose-400'}`}>
                          {keyItem.status}
                        </span>
                      </div>
                    </motion.div>
                  ))
                )}
              </div>

              {/* Gateway URL Card */}
              <div className="mt-6 bg-gradient-to-r from-indigo-50 to-blue-50 dark:from-indigo-900/30 dark:to-blue-900/30 border border-indigo-200 dark:border-indigo-800 rounded-xl p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Globe className="w-4 h-4 text-indigo-600 dark:text-indigo-400" />
                    <span className="text-sm font-medium text-slate-700 dark:text-slate-300">Gateway URL</span>
                  </div>
                  <motion.button
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => handleCopy(gatewayUrl, 'gateway')}
                    className="p-1.5 rounded-lg bg-indigo-100 dark:bg-indigo-500/20 text-indigo-600 dark:text-indigo-400 hover:bg-indigo-200 dark:hover:bg-indigo-500/30 transition-all duration-200"
                  >
                    {copiedGateway ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                  </motion.button>
                </div>
                <code className="text-sm text-indigo-600 dark:text-indigo-400 font-mono break-all mt-2">{gatewayUrl}</code>
              </div>
            </div>
          </div>
        </motion.div>
      )}

      {/* 2026 UX: Individual View — Simple Gateway Key Card */}
      {!isTeam && (
        <motion.div 
          variants={cardVariants}
          whileHover="hover"
          className="max-w-4xl mx-auto"
        >
          <div className="bg-white/90 dark:bg-slate-900/60 backdrop-blur-xl border border-slate-200/50 dark:border-slate-700/50 rounded-2xl shadow-xl dark:shadow-2xl shadow-cyan-500/10 dark:shadow-cyan-500/5 overflow-hidden">
            {/* Card Header */}
            <div className="bg-gradient-to-r from-cyan-500/5 to-teal-500/5 dark:from-cyan-500/10 dark:to-teal-500/10 px-6 py-4 border-b border-slate-200/50 dark:border-slate-700/50">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-xl bg-gradient-to-br from-cyan-500 to-teal-500 shadow-lg">
                    <Lock className="w-5 h-5 text-white" />
                  </div>
                  <div>
                    <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100">Your Gateway Key</h2>
                    <p className="text-sm text-slate-600 dark:text-slate-400">Individual account access</p>
                  </div>
                </div>
                <div className="px-3 py-1.5 rounded-full bg-gradient-to-r from-cyan-500/10 to-teal-500/10 border border-cyan-200 dark:border-cyan-800">
                  <span className="text-xs font-semibold text-cyan-700 dark:text-cyan-300 flex items-center gap-1.5">
                    <User className="w-3 h-3" />
                    Individual Plan
                  </span>
                </div>
              </div>
            </div>

            {/* Card Content */}
            <div className="p-6">
              {/* Key Display */}
              <div className="bg-slate-950 dark:bg-slate-950 border border-cyan-500/30 rounded-xl p-4 mb-6">
                <div className="flex items-center justify-between gap-4">
                  <code className="text-cyan-400 font-mono text-sm break-all leading-relaxed flex-1">
                    {user?.redeyeApiKey || 'No key available'}
                  </code>
                  <motion.button
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => user?.redeyeApiKey && handleCopy(user.redeyeApiKey, 'user-key')}
                    disabled={!user?.redeyeApiKey}
                    className="p-2 rounded-lg border border-cyan-500/20 bg-cyan-500/5 hover:bg-cyan-500/15 hover:border-cyan-500/40 transition-all duration-200 text-cyan-400 disabled:opacity-50 disabled:cursor-not-allowed"
                    title="Copy to clipboard"
                  >
                    {copiedKey === 'user-key'
                      ? <Check className="w-4 h-4 text-emerald-400" />
                      : <Copy className="w-4 h-4" />}
                  </motion.button>
                </div>
              </div>

              {/* Gateway URL */}
              <div className="bg-gradient-to-r from-indigo-50 to-blue-50 dark:from-indigo-900/30 dark:to-blue-900/30 border border-indigo-200 dark:border-indigo-800 rounded-xl p-4 mb-6">
                <div className="flex items-center justify-between gap-4">
                  <div className="flex items-center gap-2">
                    <Globe className="w-4 h-4 text-indigo-600 dark:text-indigo-400" />
                    <span className="text-sm font-medium text-slate-700 dark:text-slate-300">Gateway URL</span>
                  </div>
                  <motion.button
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => handleCopy(gatewayUrl, 'gateway')}
                    className="p-1.5 rounded-lg border border-indigo-500/20 bg-indigo-500/5 hover:bg-indigo-500/15 hover:border-indigo-500/40 transition-all duration-200 text-indigo-600 dark:text-indigo-400"
                    title="Copy Gateway URL"
                  >
                    {copiedGateway ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                  </motion.button>
                </div>
                <code className="text-sm text-indigo-600 dark:text-indigo-400 font-mono break-all mt-2">{gatewayUrl}</code>
              </div>

              {/* Upgrade CTA */}
              <div className="text-center p-6 bg-gradient-to-r from-slate-50 to-slate-100 dark:from-slate-800/50 dark:to-slate-700/50 rounded-xl border border-slate-200 dark:border-slate-600">
                <div className="text-6xl mb-4">🚀</div>
                <h3 className="text-lg font-bold text-slate-900 dark:text-slate-100 mb-2">Upgrade to Team Plan</h3>
                <p className="text-slate-600 dark:text-slate-400 mb-4">Need multiple keys for different environments or team members?</p>
                <motion.button
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                  onClick={() => alert('Upgrade flow not yet implemented')}
                  className="px-6 py-3 bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 text-white font-semibold rounded-xl shadow-lg inline-flex items-center gap-2 transition-all duration-200"
                >
                  <Users className="w-4 h-4" />
                  Upgrade to Team Plan
                </motion.button>
              </div>
            </div>
          </div>
        </motion.div>
      )}

      {/* 2026 UX: Modal with AnimatePresence */}
      <AnimatePresence>
        {isProviderModalOpen && (
          <motion.div 
            variants={modalVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 dark:bg-slate-950/80 backdrop-blur-sm"
          >
            <motion.div 
              variants={modalVariants}
              className="bg-white dark:bg-slate-900/90 border border-slate-200 dark:border-slate-800 shadow-2xl rounded-2xl w-full max-w-md overflow-hidden"
            >
              <div className="flex items-center justify-between px-6 py-4 border-b border-slate-100 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/50">
                <h3 className="text-lg font-bold text-slate-900 dark:text-slate-50 flex items-center gap-2">
                  <ShieldCheck className="w-5 h-5 text-cyan-600 dark:text-cyan-400" />
                  Add Provider Key
                </h3>
                <motion.button
                  whileHover={{ scale: 1.1 }}
                  whileTap={{ scale: 0.9 }}
                  onClick={() => setIsProviderModalOpen(false)}
                  className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors p-1"
                >
                  <X className="w-5 h-5" />
                </motion.button>
              </div>

              <form onSubmit={handleAddProvider} className="p-6 space-y-4">
                <div className="bg-cyan-50 dark:bg-cyan-500/10 p-4 rounded-xl border border-cyan-200 dark:border-cyan-800">
                  <div className="flex items-start gap-3">
                    <AlertTriangle className="w-4 h-4 text-cyan-600 dark:text-cyan-400 shrink-0 mt-0.5" />
                    <p className="text-sm text-cyan-700 dark:text-cyan-300">
                      Your API key will be encrypted before storage. We never log keys in plaintext.
                    </p>
                  </div>
                </div>

                <div className="space-y-3">
                  <div>
                    <label className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2 block">Provider</label>
                    <select
                      value={newProviderName}
                      onChange={(e) => setNewProviderName(e.target.value)}
                      className="w-full rounded-lg bg-white dark:bg-slate-950/70 border border-slate-300 dark:border-slate-700 px-4 py-3 text-sm text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 transition-all duration-200"
                    >
                      {SUPPORTED_PROVIDERS.map((provider) => (
                        <option key={provider.id} value={provider.id}>
                          {provider.name}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div>
                    <label className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2 block">API Key</label>
                    <input
                      type="password"
                      required
                      autoFocus
                      placeholder="sk-..."
                      value={newProviderKey}
                      onChange={(e) => setNewProviderKey(e.target.value)}
                      className="w-full rounded-lg bg-white dark:bg-slate-950/70 border border-slate-300 dark:border-slate-700 px-4 py-3 text-sm text-slate-900 dark:text-slate-100 font-mono placeholder:text-slate-400 dark:placeholder:text-slate-600 focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:border-cyan-500 transition-all duration-200"
                    />
                  </div>
                </div>

                <div className="flex gap-3 justify-end pt-2">
                  <motion.button
                    type="button"
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => setIsProviderModalOpen(false)}
                    className="rounded-lg px-4 py-2 text-sm font-semibold text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-all duration-200"
                  >
                    Cancel
                  </motion.button>
                  <motion.button
                    type="submit"
                    disabled={submitting}
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    className="inline-flex items-center justify-center gap-2 rounded-lg bg-gradient-to-r from-cyan-500 to-teal-500 hover:from-cyan-400 hover:to-teal-400 disabled:opacity-50 disabled:cursor-not-allowed px-5 py-2 text-sm font-semibold text-white shadow-lg transition-all duration-200"
                  >
                    {submitting ? (
                      <>
                        <Loader2 className="w-4 h-4 animate-spin" />
                        Adding...
                      </>
                    ) : (
                      'Add Provider Key'
                    )}
                  </motion.button>
                </div>
              </form>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* 2026 UX: Generate Key Modal */}
      <AnimatePresence>
        {isKeyModalOpen && isTeam && (
          <motion.div 
            variants={modalVariants}
            initial="hidden"
            animate="visible"
            exit="exit"
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 dark:bg-slate-950/80 backdrop-blur-sm"
          >
            <motion.div 
              variants={modalVariants}
              className="bg-white dark:bg-slate-900/90 border border-rose-200 dark:border-rose-800 shadow-2xl rounded-2xl w-full max-w-md overflow-hidden"
            >
              <div className="flex items-center justify-between px-6 py-4 border-b border-rose-100 dark:border-rose-800 bg-rose-50 dark:bg-rose-950/20">
                <h3 className="text-lg font-bold text-slate-900 dark:text-slate-50 flex items-center gap-2">
                  <ShieldCheck className="w-5 h-5 text-rose-600 dark:text-rose-400" />
                  Generate Virtual API Key
                </h3>
                <motion.button
                  whileHover={{ scale: 1.1 }}
                  whileTap={{ scale: 0.9 }}
                  onClick={() => setIsKeyModalOpen(false)}
                  className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors p-1"
                >
                  <X className="w-5 h-5" />
                </motion.button>
              </div>

              <form onSubmit={handleGenerate} className="p-6">
                <div className="bg-rose-50 dark:bg-rose-500/10 p-4 rounded-xl border border-rose-200 dark:border-rose-800 mb-6">
                  <div className="flex items-start gap-3">
                    <AlertTriangle className="w-4 h-4 text-rose-600 dark:text-rose-400 shrink-0 mt-0.5" />
                    <p className="text-sm text-rose-700 dark:text-rose-300">
                      For security, your new key will only be shown once. Have your clipboard ready.
                    </p>
                  </div>
                </div>

                <div className="space-y-3 mb-6">
                  <div>
                    <label className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2 block">Key Name</label>
                    <input
                      type="text"
                      required
                      autoFocus
                      placeholder="e.g. Production Frontend App"
                      value={newKeyName}
                      onChange={(e) => setNewKeyName(e.target.value)}
                      className="w-full rounded-lg bg-white dark:bg-slate-950/70 border border-slate-300 dark:border-rose-900/50 px-4 py-3 text-sm text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-600 focus:outline-none focus:ring-2 focus:ring-rose-500 focus:border-rose-500 transition-all duration-200"
                    />
                  </div>
                </div>

                <div className="flex gap-3 justify-end">
                  <motion.button
                    type="button"
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => setIsKeyModalOpen(false)}
                    className="rounded-lg px-4 py-2 text-sm font-semibold text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-all duration-200"
                  >
                    Cancel
                  </motion.button>
                  <motion.button
                    type="submit"
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    className="inline-flex items-center justify-center gap-2 rounded-lg bg-gradient-to-r from-rose-500 to-pink-500 hover:from-rose-400 hover:to-pink-400 px-5 py-2 text-sm font-semibold text-white shadow-lg transition-all duration-200"
                  >
                    Generate Key
                  </motion.button>
                </div>
              </form>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
