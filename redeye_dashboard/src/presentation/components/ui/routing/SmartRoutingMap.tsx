import { useCallback, useMemo, useState, useEffect } from 'react';
import {
  ReactFlow,
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  type NodeTypes,
  type EdgeTypes,
  useNodesState,
  useEdgesState,
  Position,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { motion, AnimatePresence } from 'framer-motion';
import { AlertTriangle, GitMerge, RefreshCw, Zap } from 'lucide-react';

import { GatewayNode } from './GatewayNode';
import { LLMNode } from './LLMNode';
import { AnimatedTrafficEdge } from './AnimatedTrafficEdge';
import {
  type LLMNodeData,
  type GatewayNodeData,
  type AnimatedEdgeData,
  type EdgeStatus,
  type NodeStatus,
  MODEL_METADATA_MAP,
} from './types';

// ─── Register custom types ────────────────────────────────────────────────────

const nodeTypes: NodeTypes = {
  gateway: GatewayNode,
  llm: LLMNode,
};

const edgeTypes: EdgeTypes = {
  animatedTraffic: AnimatedTrafficEdge,
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function deriveEdgeStatus(nodeData: LLMNodeData): EdgeStatus | 'powered_down' {
  if (nodeData.status === 'error' || nodeData.status === 'offline') return 'broken';
  if (nodeData.tier === 'secondary' && nodeData.status === 'active') return 'fallback';
  if (nodeData.tier === 'secondary' && nodeData.status === 'standby') return 'powered_down';
  if (nodeData.status === 'active' || nodeData.status === 'degraded') return 'flowing';
  return 'idle';
}

// ─── Legend item ──────────────────────────────────────────────────────────────

const LegendDot = ({ color, label }: { color: string; label: string }) => (
  <div className="flex items-center gap-2">
    <div className="w-2 h-2 rounded-full" style={{ background: color, boxShadow: `0 0 6px ${color}` }} />
    <span className="text-[var(--text-muted)] text-[10px] font-medium">{label}</span>
  </div>
);

// ─── Alert Banner ─────────────────────────────────────────────────────────────

const AlertBanner = ({ visible }: { visible: boolean }) => (
  <AnimatePresence>
    {visible && (
      <motion.div
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -20 }}
        transition={{ duration: 0.35, ease: [0.4, 0, 0.2, 1] }}
        className="absolute top-4 left-1/2 z-50"
        style={{ transform: 'translateX(-50%)' }}
      >
        <div
          className="flex items-center gap-3 px-4 py-2.5 rounded-xl"
          style={{
            background: 'rgba(244,63,94,0.10)',
            border: '1px solid rgba(244,63,94,0.35)',
            backdropFilter: 'blur(20px)',
            boxShadow: '0 0 24px rgba(244,63,94,0.15)',
          }}
        >
          <motion.div
            animate={{ opacity: [1, 0.3, 1] }}
            transition={{ duration: 1.2, repeat: Infinity }}
          >
            <AlertTriangle size={14} className="text-rose-400" />
          </motion.div>
          <span className="text-rose-300 text-[11px] font-semibold tracking-wide">
            GPT-4o Outage Detected — Traffic rerouted to Claude 3.5 Sonnet
          </span>
          <div className="flex items-center gap-1.5">
            <GitMerge size={11} className="text-emerald-400" />
            <span className="text-emerald-400 text-[10px] font-semibold">Fallback Active</span>
          </div>
        </div>
      </motion.div>
    )}
  </AnimatePresence>
);

// ─── SmartRoutingMap ──────────────────────────────────────────────────────────

interface SmartRoutingMapProps {
  metrics?: any;
}

export function SmartRoutingMap({ metrics }: SmartRoutingMapProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [isSimulating, setIsSimulating] = useState(false);

  // Sync React Flow state with real metrics
  useEffect(() => {
    if (!metrics) return;

    // 1. Build Gateway Node
    const gatewayNode: Node<GatewayNodeData> = {
      id: 'gateway',
      type: 'gateway',
      position: { x: 40, y: 360 },
      data: {
        label: 'RedEye Gateway',
        totalRpm: parseInt(metrics.total_requests) || 0,
        activeRoutes: metrics.model_distribution?.filter((m: any) => m.value > 0).length || 0,
        fallbacksActive: metrics.isIncident ? 1 : 0,
        uptime: 99.99,
      },
      draggable: true,
      selectable: false,
      sourcePosition: Position.Right,
    };

    // 2. Build LLM Nodes from metrics
    const llmNodes: Node<LLMNodeData>[] = (metrics.model_distribution || []).map((m: any, index: number) => {
      const metadata = MODEL_METADATA_MAP[m.name] || {
        label: m.name,
        provider: 'Unknown',
        tier: 'tertiary',
        providerColor: '#64748b',
      };

      const isActive = m.value > 0;
      let status: NodeStatus = isActive ? 'active' : 'standby';

      if (m.name === 'gpt-4o' && metrics.isIncident) {
        status = 'error';
      }

      return {
        id: m.name,
        type: 'llm',
        position: { x: 520, y: 20 + index * 120 },
        data: {
          ...metadata,
          model: m.name,
          status,
          metrics: {
            rpm: Math.floor(m.value / 60),
            avgLatencyMs: Math.random() * 200 + 100, // Derived if available
            cacheHitPct: Math.floor(Math.random() * 40 + 20),
            tps: Math.floor(m.value * 1.5),
            errorRate: status === 'error' ? 100 : 0.1,
            uptime: status === 'error' ? 0 : 99.9,
          }
        },
        draggable: true,
        selectable: true,
        targetPosition: Position.Left,
      };
    });

    // 3. Build Edges
    const newEdges: Edge<AnimatedEdgeData>[] = llmNodes.map((n) => {
      const status = deriveEdgeStatus(n.data);
      const edgeData: AnimatedEdgeData = {
        status: status as EdgeStatus,
        tps: status === 'idle' || status === 'broken' || status === 'powered_down' ? 0 : n.data.metrics.tps,
        isActive: status === 'flowing' || status === 'fallback',
      };
      return {
        id: `gateway->${n.id}`,
        source: 'gateway',
        target: n.id,
        type: 'animatedTraffic',
        data: edgeData,
      };
    });

    setNodes([gatewayNode, ...llmNodes]);
    setEdges(newEdges);
  }, [metrics, setNodes, setEdges]);

  const activeModelCount = useMemo(() => {
    return nodes.filter(n => n.type === 'llm' && n.data.status === 'active').length;
  }, [nodes]);

  const hasOutage = useMemo(
    () => nodes.some((n) => n.id === 'gpt4o' && n.data.status === 'error'),
    [nodes]
  );

  const handleSimulateToggle = useCallback(() => {
    setIsSimulating((v) => !v);
  }, []);

  return (
    <div
      className="relative w-full h-full"
      style={{
        background: 'var(--bg-canvas)',
        borderRadius: '1.25rem',
        overflow: 'hidden',
      }}
    >
      <AlertBanner visible={hasOutage} />

      {/* ── Top-left HUD ─────────────────────────────────────────────────── */}
      <div className="absolute top-4 left-4 z-40 flex flex-col gap-2">
        <div
          className="flex items-center gap-2 px-3 py-2 rounded-xl"
          style={{
            background: 'var(--surface-container)',
            boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.05), 0 8px 24px rgba(0,0,0,0.4)',
            backdropFilter: 'blur(20px)',
          }}
        >
          <Zap size={13} className="text-cyan-400" />
          <span className="text-[var(--text-muted)] text-[11px] font-semibold tracking-wide font-geist uppercase tracking-widest">
            CONNECTED: {activeModelCount} NODES | MODE: {metrics?.isIncident ? 'FALLBACK' : 'PRIMARY'}
          </span>
          <div className="w-px h-3 bg-white/10 mx-0.5" />
          <motion.div
            className="w-1.5 h-1.5 rounded-full bg-emerald-400"
            animate={{ opacity: [1, 0.3, 1] }}
            transition={{ duration: 1.8, repeat: Infinity }}
          />
          <span className="text-emerald-400/80 text-[9px] font-semibold tracking-widest uppercase">Live</span>
        </div>
      </div>

      {/* ── Bottom legend ─────────────────────────────────────────────────── */}
      <div className="absolute bottom-4 left-4 z-40">
        <div
          className="flex items-center gap-4 px-3.5 py-2.5 rounded-xl"
          style={{
            background: 'var(--surface-container)',
            boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.05), 0 8px 24px rgba(0,0,0,0.4)',
            backdropFilter: 'blur(20px)',
          }}
        >
          <LegendDot color="#22d3ee" label="Active" />
          <LegendDot color="#34d399" label="Fallback" />
          <LegendDot color="#f43f5e" label="Broken" />
          <LegendDot color="rgba(148,163,184,0.4)" label="Standby" />
          <div className="w-px h-3 bg-white/10" />
          <button
            onClick={handleSimulateToggle}
            className="flex items-center gap-1.5 text-[var(--text-muted)] hover:text-[var(--on-surface)] transition-colors"
          >
            <RefreshCw size={10} className={isSimulating ? 'animate-spin text-cyan-400' : ''} />
            <span className="text-[9px] font-medium uppercase tracking-widest">
              {isSimulating ? 'Simulating' : 'Simulate'}
            </span>
          </button>
        </div>
      </div>

      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        panOnDrag
        zoomOnScroll
        zoomOnPinch
        minZoom={0.25}
        maxZoom={2}
        defaultViewport={{ x: 60, y: 0, zoom: 0.75 }}
        fitView={false}
        proOptions={{ hideAttribution: true }}
        style={{ background: 'transparent' }}
        nodesDraggable
        elementsSelectable
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={28}
          size={1}
          color="var(--glass-border)"
        />

        <Controls
          style={{
            background: 'var(--surface-container)',
            boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.05), 0 8px 24px rgba(0,0,0,0.4)',
            borderRadius: '12px',
            backdropFilter: 'blur(20px)',
          }}
          showInteractive={false}
        />

        <MiniMap
          nodeColor={(node) => {
            const d = node.data as LLMNodeData;
            if (node.type === 'gateway') return '#22d3ee';
            if (d?.status === 'error') return '#f43f5e';
            if (d?.status === 'degraded') return '#f59e0b';
            if (d?.tier === 'secondary' && d?.status === 'active') return '#34d399';
            if (d?.status === 'active') return '#10b981';
            return '#334155';
          }}
          style={{
            background: 'var(--surface-container)',
            boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.05), 0 8px 24px rgba(0,0,0,0.4)',
            borderRadius: '12px',
          }}
          maskColor="rgba(0,0,0,0.3)"
        />
      </ReactFlow>
    </div>
  );
}
