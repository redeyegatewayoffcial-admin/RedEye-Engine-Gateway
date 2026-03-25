// Presentation Dashboard — ApiKeysView
// API Keys Management using Dark Red / Neon Crimson aesthetic

import { useState } from 'react';
import { Key, Plus, Trash2, X, AlertTriangle, ShieldCheck, Copy, Check, Globe, Loader2 } from 'lucide-react';
import useSWR from 'swr';

export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  created_at: string;
  status: string;
}

const fetcher = async (url: string) => {
  const token = localStorage.getItem('re_token');
  if (!token) throw new Error("No authentication token found");
  const res = await fetch(url, { headers: { 'Authorization': `Bearer ${token}` } });
  if (!res.ok) throw new Error(`HTTP error! status: ${res.status}`);
  return res.json();
};

export function ApiKeysView() {
  const { data: keys, error, isLoading, mutate: _mutate } = useSWR<ApiKey[]>(
    'http://localhost:8084/v1/auth/api-keys',
    fetcher
  );
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [newKeyName, setNewKeyName] = useState('');
  const [copiedGateway, setCopiedGateway] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

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
    // Optimistic UI update could go here, but for now we just show a stub or rely on mutate
    alert("Revoke endpoint not yet implemented");
  };

  const handleGenerate = (e: React.FormEvent) => {
    e.preventDefault();
    if (!newKeyName.trim()) return;

    // Usually this invokes a POST /api-keys to generate one.
    // For now we just close the modal.
    alert("Generate endpoint not yet implemented");
    setIsModalOpen(false);
  };

  const activeKeysCount = keys?.filter(k => k.status === 'Active').length || 0;
  const activeKey = keys?.find(k => k.status === 'Active');

  return (
    <div className="space-y-6">
      {/* Gateway Info Card */}
      <div className="glass-panel bg-slate-900/50 border border-slate-800 rounded-xl p-6 space-y-4">
        <h2 className="text-lg font-bold text-slate-50 flex items-center gap-2">
          <Globe className="w-5 h-5 text-indigo-400" />
          Gateway Connection
        </h2>
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="bg-slate-950/70 rounded-lg border border-slate-800 p-4">
            <p className="text-xs font-medium text-slate-400 mb-1.5">Gateway URL</p>
            <div className="flex items-center gap-2">
              <code className="text-sm text-indigo-400 font-mono break-all flex-1">{gatewayUrl}</code>
              <button
                onClick={() => handleCopy(gatewayUrl, 'gateway')}
                className="flex-none p-1.5 rounded-md hover:bg-slate-800 transition-colors text-slate-400 hover:text-slate-200"
                title="Copy Gateway URL"
              >
                {copiedGateway ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
              </button>
            </div>
          </div>
          {activeKeysCount > 0 && activeKey && (
            <div className="bg-slate-950/70 rounded-lg border border-slate-800 p-4">
              <p className="text-xs font-medium text-slate-400 mb-1.5">Your API Key (Hash)</p>
              <div className="flex items-center gap-2">
                <code className="text-sm text-rose-400 font-mono break-all flex-1">
                  {activeKey.key_hash.substring(0, 16)}...
                </code>
                <button
                  onClick={() => handleCopy(activeKey.key_hash ?? '', 'active-key')}
                  className="flex-none p-1.5 rounded-md hover:bg-slate-800 transition-colors text-slate-400 hover:text-slate-200"
                  title="Copy API Key Hash"
                >
                  {copiedKey === 'active-key' ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Header section */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2 text-slate-50">
            <Key className="w-6 h-6 text-rose-500" />
            API Keys Management
          </h1>
          <p className="text-sm text-slate-400 mt-1">
            Generate and manage your RedEye Gateway API keys to access LLM providers.
          </p>
        </div>
        <button
          onClick={() => setIsModalOpen(true)}
          className="inline-flex items-center justify-center gap-2 rounded-lg bg-rose-600 hover:bg-rose-500 px-4 py-2.5 text-sm font-semibold text-white shadow-[0_0_20px_rgba(225,29,72,0.25)] transition-all duration-200"
        >
          <Plus className="w-4 h-4" />
          Generate New Key
        </button>
      </div>

      {/* Table section */}
      <div className="glass-panel bg-rose-950/20 border border-rose-900/40 rounded-xl overflow-hidden shadow-xl">
        <div className="overflow-x-auto custom-scrollbar">
          <table className="w-full text-left text-sm text-slate-300">
            <thead className="bg-rose-950/40 text-xs uppercase text-slate-400 border-b border-rose-900/40">
              <tr>
                <th scope="col" className="px-6 py-4 font-semibold tracking-wider">Key Name</th>
                <th scope="col" className="px-6 py-4 font-semibold tracking-wider">Masked Key</th>
                <th scope="col" className="px-6 py-4 font-semibold tracking-wider">Created</th>
                <th scope="col" className="px-6 py-4 font-semibold tracking-wider">Status</th>
                <th scope="col" className="px-6 py-4 font-semibold tracking-wider text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-rose-900/30">
              {isLoading ? (
                <tr>
                  <td colSpan={5} className="px-6 py-8 text-center text-slate-500">
                    <Loader2 className="w-5 h-5 animate-spin mx-auto text-rose-500 mb-2" />
                    Loading API keys...
                  </td>
                </tr>
              ) : error ? (
                <tr>
                  <td colSpan={5} className="px-6 py-8 text-center text-rose-400">
                    Failed to fetch API keys. Is the Auth service running?
                  </td>
                </tr>
              ) : !keys || keys.length === 0 ? (
                <tr>
                  <td colSpan={5} className="px-6 py-8 text-center text-slate-500">
                    No API keys found. Generate one to get started.
                  </td>
                </tr>
              ) : (
                keys.map((keyItem) => (
                  <tr key={keyItem.id} className="hover:bg-rose-900/10 transition-colors">
                    <td className="px-6 py-4 font-medium text-slate-200">{keyItem.name}</td>
                    <td className="px-6 py-4 font-mono text-xs text-rose-400/90">{keyItem.key_hash.substring(0, 16)}...</td>
                    <td className="px-6 py-4 text-slate-400">
                      {new Date(keyItem.created_at).toLocaleDateString(undefined, {
                        year: 'numeric', month: 'short', day: 'numeric'
                      })}
                    </td>
                    <td className="px-6 py-4">
                      {keyItem.status === 'Active' ? (
                        <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2.5 py-1 text-xs font-semibold text-emerald-400 border border-emerald-500/20">
                          <span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                          Active
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1.5 rounded-full bg-rose-500/10 px-2.5 py-1 text-xs font-semibold text-rose-400 border border-rose-500/20">
                          Revoked
                        </span>
                      )}
                    </td>
                    <td className="px-6 py-4 text-right">
                      {keyItem.status === 'Active' && (
                        <button
                          onClick={() => handleRevoke(keyItem.id)}
                          className="inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium text-rose-400 hover:text-white hover:bg-rose-600/90 transition-all border border-transparent hover:border-rose-500 focus:outline-none focus:ring-2 focus:ring-rose-500/50"
                          title="Revoke Key"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                          Revoke
                        </button>
                      )}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Generate Modal (Mock) */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm">
          <div className="glass-panel bg-slate-900/90 border border-rose-900 shadow-2xl rounded-2xl w-full max-w-md overflow-hidden relative animate-in fade-in zoom-in-95 duration-200">
            <div className="flex items-center justify-between px-6 py-4 border-b border-rose-900/50 bg-rose-950/20">
              <h2 className="text-lg font-bold text-slate-50 flex items-center gap-2">
                <ShieldCheck className="w-5 h-5 text-rose-400" />
                Generate API Key
              </h2>
              <button
                onClick={() => setIsModalOpen(false)}
                className="text-slate-400 hover:text-slate-200 transition-colors p-1"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            <form onSubmit={handleGenerate} className="p-6">
              <p className="text-sm text-slate-400 mb-6 flex items-start gap-2 bg-rose-950/30 p-3 rounded-lg border border-rose-900/30">
                <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0 mt-0.5" />
                For security, your new key will only be shown once. Have your clipboard ready.
              </p>

              <div className="space-y-2 mb-6">
                <label className="text-sm font-medium text-slate-300">
                  Key Name
                </label>
                <input
                  type="text"
                  required
                  autoFocus
                  placeholder="e.g. Production Frontend App"
                  value={newKeyName}
                  onChange={(e) => setNewKeyName(e.target.value)}
                  className="w-full rounded-lg bg-slate-950/70 border border-rose-900/50 px-3 py-2.5 text-sm text-slate-100 placeholder:text-slate-600 focus:outline-none focus:ring-1 focus:ring-rose-500 focus:border-rose-500 transition-colors"
                />
              </div>

              <div className="flex gap-3 justify-end">
                <button
                  type="button"
                  onClick={() => setIsModalOpen(false)}
                  className="rounded-lg px-4 py-2 text-sm font-semibold text-slate-300 hover:text-white hover:bg-slate-800 transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="inline-flex items-center justify-center gap-2 rounded-lg bg-rose-600 hover:bg-rose-500 px-5 py-2 text-sm font-semibold text-white shadow-[0_0_15px_rgba(225,29,72,0.3)] transition-all duration-200"
                >
                  Generate Key
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
