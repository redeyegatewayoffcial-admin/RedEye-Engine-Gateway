import React, { useState, useEffect, useRef, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Search, Command, Key, CreditCard, 
  Activity, Plus, ChevronRight 
} from 'lucide-react';

interface OmniPaletteProps {
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
}

interface CommandItem {
  id: string;
  label: string;
  category: 'Navigate' | 'Action';
  icon: React.ElementType;
  action: () => void;
  shortcut?: string;
}

export function OmniPalette({ isOpen, setIsOpen }: OmniPaletteProps) {
  const navigate = useNavigate();
  const [search, setSearch] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands: CommandItem[] = useMemo(() => [
    { 
      id: 'api-keys', 
      label: 'Navigate: API Keys', 
      category: 'Navigate', 
      icon: Key, 
      action: () => navigate('/dashboard/api-keys') 
    },
    { 
      id: 'billing', 
      label: 'Navigate: Cost & Billing', 
      category: 'Navigate', 
      icon: CreditCard, 
      action: () => navigate('/dashboard/billing') 
    },
    { 
      id: 'outage', 
      label: 'Action: Simulate Outage', 
      category: 'Action', 
      icon: Activity, 
      action: () => alert('Simulating outage... System health monitoring active.') 
    },
    { 
      id: 'new-key', 
      label: 'Action: Create New API Key', 
      category: 'Action', 
      icon: Plus, 
      action: () => alert('Navigating to API Key creation flow...') 
    },
  ], [navigate]);

  const filteredCommands = useMemo(() => {
    return commands.filter(cmd => 
      cmd.label.toLowerCase().includes(search.toLowerCase())
    );
  }, [search, commands]);

  // Handle global Cmd+K / Ctrl+K
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setIsOpen(!isOpen);
      }
      if (e.key === 'Escape') {
        setIsOpen(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, setIsOpen]);

  // Focus input when opened
  useEffect(() => {
    if (isOpen) {
      setSearch('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 10);
    }
  }, [isOpen]);

  // Keyboard navigation within the palette
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex(prev => (prev + 1) % filteredCommands.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex(prev => (prev - 1 + filteredCommands.length) % filteredCommands.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredCommands[selectedIndex]) {
        filteredCommands[selectedIndex].action();
        setIsOpen(false);
      }
    }
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setIsOpen(false)}
            className="fixed inset-0 bg-black/60 backdrop-blur-[10px] z-[100]"
          />

          {/* Palette Container */}
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: -20 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: -20 }}
            transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
            className="fixed top-[15%] left-1/2 -translate-x-1/2 w-full max-w-[640px] z-[101] px-4"
          >
            <div 
              className="relative overflow-hidden rounded-2xl"
              style={{
                backgroundColor: 'rgba(20, 20, 20, 0.6)',
                backdropFilter: 'blur(40px) saturate(200%)',
                WebkitBackdropFilter: 'blur(40px) saturate(200%)',
                boxShadow: `
                  0 0 0 1px rgba(255, 255, 255, 0.05),
                  0 32px 64px rgba(0, 0, 0, 0.6),
                  inset 0 1px 1px rgba(34, 211, 238, 0.3)
                `
              }}
            >
              {/* Search Input */}
              <div className="flex items-center gap-4 px-5 py-4 border-b border-white/5">
                <Search className="w-5 h-5 text-cyan-400 opacity-80" />
                <input
                  ref={inputRef}
                  type="text"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="Type a command or search..."
                  className="flex-1 bg-transparent border-none outline-none text-xl font-geist text-white placeholder:text-white/20"
                />
                <div className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-white/5 border border-white/10">
                  <span className="text-[10px] font-jetbrains text-white/40">ESC to close</span>
                </div>
              </div>

              {/* Results */}
              <div className="max-h-[400px] overflow-y-auto custom-scrollbar py-2">
                {filteredCommands.length > 0 ? (
                  filteredCommands.map((cmd, index) => (
                    <button
                      key={cmd.id}
                      onClick={() => {
                        cmd.action();
                        setIsOpen(false);
                      }}
                      onMouseEnter={() => setSelectedIndex(index)}
                      className={`w-full flex items-center justify-between px-5 py-3.5 transition-all duration-200 group relative ${
                        selectedIndex === index ? 'bg-cyan-500/10' : ''
                      }`}
                    >
                      <div className="flex items-center gap-4">
                        <div className={`p-2 rounded-lg transition-colors ${
                          selectedIndex === index ? 'bg-cyan-500/20 text-cyan-400' : 'bg-white/5 text-white/40'
                        }`}>
                          <cmd.icon className="w-4 h-4" />
                        </div>
                        <span className={`text-sm font-jetbrains tracking-tight transition-colors ${
                          selectedIndex === index ? 'text-white' : 'text-white/60'
                        }`}>
                          {cmd.label}
                        </span>
                      </div>

                      {selectedIndex === index && (
                        <motion.div 
                          layoutId="active-glow"
                          className="flex items-center gap-2 text-cyan-400"
                        >
                          <span className="text-[10px] font-jetbrains font-bold uppercase tracking-wider opacity-60">Execute</span>
                          <ChevronRight className="w-4 h-4" />
                        </motion.div>
                      )}

                      {/* Selection Glow Indicator */}
                      {selectedIndex === index && (
                        <div 
                          className="absolute inset-y-0 left-0 w-1 bg-cyan-400 shadow-[0_0_12px_rgba(34,211,238,0.8)]"
                        />
                      )}
                    </button>
                  ))
                ) : (
                  <div className="px-5 py-12 text-center">
                    <p className="text-white/30 font-geist text-sm">No commands found for "{search}"</p>
                  </div>
                )}
              </div>

              {/* Footer */}
              <div className="px-5 py-3 bg-white/5 border-t border-white/5 flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-1.5">
                    <kbd className="px-1.5 py-0.5 rounded bg-white/10 text-[9px] font-jetbrains text-white/60">↑↓</kbd>
                    <span className="text-[10px] text-white/40 font-geist">to navigate</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <kbd className="px-1.5 py-0.5 rounded bg-white/10 text-[9px] font-jetbrains text-white/60">↵</kbd>
                    <span className="text-[10px] text-white/40 font-geist">to select</span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <Command className="w-3 h-3 text-white/20" />
                  <span className="text-[10px] text-white/20 font-jetbrains uppercase tracking-widest">RedEye Terminal</span>
                </div>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
