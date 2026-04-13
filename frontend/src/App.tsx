import React, { useState, useEffect } from 'react';
import { Server, Terminal, Activity, ArrowRight, Box, X, Play, Plus, Cpu } from 'lucide-react';
import { cn } from '@/lib/utils';

interface Daemon {
  id: string;
  status: string;
  tools_count: number;
  tools: string[];
}

interface MCPTool {
  name: string;
  description: string;
  daemon_id: string;
  inputSchema?: any;
}

const API_BASE = '/v1';
const HEADERS = {
  'Authorization': 'Bearer DUMMY_TOKEN', // Require_auth is false in our dev config, but providing anyway
};

export default function Dashboard() {
  const [daemons, setDaemons] = useState<Daemon[]>([]);
  const [tools, setTools] = useState<MCPTool[]>([]);
  const [activeTab, setActiveTab] = useState<'daemons' | 'tools'>('daemons');
  const [loading, setLoading] = useState(true);

  // Execution Modal State
  const [selectedTool, setSelectedTool] = useState<MCPTool | null>(null);
  const [toolArgs, setToolArgs] = useState<string>('{\n  \n}');
  const [execResult, setExecResult] = useState<string | null>(null);
  const [isExecuting, setIsExecuting] = useState(false);

  // Add Server Modal State
  const [selectedDaemon, setSelectedDaemon] = useState<Daemon | null>(null);
  const [newServerName, setNewServerName] = useState<string>('sqlite-server');
  const [newServerCmd, setNewServerCmd] = useState<string>('npx');
  const [newServerArgs, setNewServerArgs] = useState<string>('-y, @modelcontextprotocol/server-sqlite, test.db');
  const [isAddingServer, setIsAddingServer] = useState(false);
  const [addServerError, setAddServerError] = useState<string | null>(null);

  const fetchData = async () => {
    setLoading(true);
    try {
      const [dRes, tRes] = await Promise.all([
        fetch(`${API_BASE}/daemons`, { headers: HEADERS }),
        fetch(`${API_BASE}/tools`, { headers: HEADERS })
      ]);
      
      if (dRes.ok) setDaemons(await dRes.json());
      if (tRes.ok) setTools(await tRes.json());
    } catch (e) {
      console.error('Failed to fetch:', e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleAddServer = async () => {
    if (!selectedDaemon) return;
    setIsAddingServer(true);
    setAddServerError(null);

    try {
      const argsArray = newServerArgs.split(',').map(a => a.trim()).filter(a => a);
      const res = await fetch(`${API_BASE}/daemons/${selectedDaemon.id}/servers`, {
        method: 'POST',
        headers: {
          ...HEADERS,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          name: newServerName,
          command: newServerCmd,
          args: argsArray,
          env: {}
        })
      });

      const data = await res.json();
      if (!res.ok) {
        throw new Error(data.error || 'Failed to add server');
      }
      
      // Success, close modal and wait for polling to update the UI
      setSelectedDaemon(null);
    } catch (err: any) {
      setAddServerError(`Error: ${err.message || err.toString()}`);
    } finally {
      setIsAddingServer(false);
    }
  };

  const handleExecute = async () => {
    if (!selectedTool) return;
    setIsExecuting(true);
    setExecResult(null);

    try {
      let parsedArgs = undefined;
      if (toolArgs.trim() && toolArgs.trim() !== '{}') {
        parsedArgs = JSON.parse(toolArgs);
      }

      const res = await fetch(`${API_BASE}/tools/call`, {
        method: 'POST',
        headers: {
          ...HEADERS,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          name: selectedTool.name,
          arguments: parsedArgs
        })
      });

      const data = await res.json();
      setExecResult(JSON.stringify(data, null, 2));
    } catch (err: any) {
      setExecResult(`Error: ${err.message || err.toString()}`);
    } finally {
      setIsExecuting(false);
    }
  };

  return (
    <div className="min-h-screen bg-[#0a0a0a] text-zinc-100 font-mono">
      {/* Header */}
      <header className="border-b border-zinc-800 bg-black/50 backdrop-blur-xl sticky top-0 z-50">
        <div className="container mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Terminal className="w-5 h-5 text-emerald-400" />
            <h1 className="font-bold tracking-tight text-lg">MCP Gateway Hub</h1>
          </div>
          <div className="flex items-center gap-6 text-sm text-zinc-400">
            <div className="flex items-center gap-2">
              <Activity className="w-4 h-4 text-emerald-400" />
              <span>{daemons.length} Daemons</span>
            </div>
            <div className="flex items-center gap-2">
              <Box className="w-4 h-4 text-blue-400" />
              <span>{tools.length} Tools</span>
            </div>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-6 py-8">
        {/* Navigation Tabs */}
        <div className="flex gap-4 mb-8 border-b border-zinc-800">
          <button
            onClick={() => setActiveTab('daemons')}
            className={cn(
              "pb-3 px-1 text-sm transition-colors border-b-2 font-medium",
              activeTab === 'daemons' 
                ? "border-emerald-400 text-emerald-400" 
                : "border-transparent text-zinc-500 hover:text-zinc-300"
            )}
          >
            Daemons
          </button>
          <button
            onClick={() => setActiveTab('tools')}
            className={cn(
              "pb-3 px-1 text-sm transition-colors border-b-2 font-medium",
              activeTab === 'tools' 
                ? "border-blue-400 text-blue-400" 
                : "border-transparent text-zinc-500 hover:text-zinc-300"
            )}
          >
            Registered Tools
          </button>
        </div>

        {/* Content */}
        {loading && daemons.length === 0 ? (
          <div className="flex justify-center py-20 text-zinc-500">
            <Activity className="w-6 h-6 animate-pulse" />
          </div>
        ) : (
          <div className="animate-in fade-in duration-500">
            {activeTab === 'daemons' ? (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {daemons.map(d => (
                  <div key={d.id} className="group p-6 rounded-xl bg-zinc-900/50 border border-zinc-800 hover:border-zinc-700 transition-all flex flex-col">
                    <div className="flex items-start justify-between mb-4">
                      <div className="flex items-center gap-3">
                        <div className="p-2 rounded-lg bg-emerald-400/10 text-emerald-400">
                          <Server className="w-5 h-5" />
                        </div>
                        <div>
                          <h3 className="font-medium text-zinc-100">{d.id}</h3>
                          <p className="text-xs text-zinc-500 mt-1 flex items-center gap-1">
                            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                            {d.status}
                          </p>
                        </div>
                      </div>
                    </div>
                    <div className="pt-4 border-t border-zinc-800/50 flex-1">
                      <p className="text-sm text-zinc-400 mb-4">
                        <span className="text-zinc-100 font-medium">{d.tools_count}</span> tools mounted
                      </p>
                      <button 
                        onClick={() => setSelectedDaemon(d)}
                        className="w-full py-2 flex items-center justify-center gap-2 rounded-md bg-zinc-800 hover:bg-zinc-700 text-sm text-zinc-300 transition-colors mt-auto"
                      >
                        <Plus className="w-4 h-4" /> Add MCP Server
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="space-y-4">
                {tools.map(t => (
                  <div key={`${t.daemon_id}-${t.name}`} className="p-5 rounded-lg bg-zinc-900/30 border border-zinc-800 hover:bg-zinc-900/50 transition-colors">
                    <div className="flex items-start justify-between">
                      <div>
                        <div className="flex items-center gap-3 mb-2">
                          <h3 className="font-semibold text-blue-400">{t.name}</h3>
                          <span className="px-2 py-0.5 rounded text-xs bg-zinc-800 text-zinc-400 border border-zinc-700">
                            {t.daemon_id}
                          </span>
                        </div>
                        <p className="text-sm text-zinc-400 max-w-3xl leading-relaxed">
                          {t.description || "No description provided."}
                        </p>
                      </div>
                      <button 
                        onClick={() => {
                          setSelectedTool(t);
                          setToolArgs('{\n  \n}');
                          setExecResult(null);
                        }}
                        className="flex items-center gap-2 px-3 py-1.5 rounded-md bg-zinc-800 hover:bg-zinc-700 text-sm text-zinc-300 transition-colors"
                      >
                        Execute <ArrowRight className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </main>

      {/* Add Server Modal */}
      {selectedDaemon && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200">
          <div className="bg-[#111] border border-zinc-800 rounded-xl w-full max-w-md overflow-hidden flex flex-col shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-zinc-800 bg-zinc-900/50">
              <h2 className="font-semibold text-zinc-100 flex items-center gap-2">
                <Cpu className="w-4 h-4 text-emerald-400" />
                Add MCP Server to <span className="text-zinc-400">{selectedDaemon.id}</span>
              </h2>
              <button 
                onClick={() => setSelectedDaemon(null)} 
                className="text-zinc-500 hover:text-zinc-300 transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <div className="p-6 space-y-4">
              <div>
                <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-2 block">
                  Server Name
                </label>
                <input
                  type="text"
                  value={newServerName}
                  onChange={(e) => setNewServerName(e.target.value)}
                  className="w-full bg-black border border-zinc-800 rounded-lg p-2.5 text-sm text-zinc-300 focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/50 transition-all"
                  placeholder="e.g. sqlite-server"
                />
              </div>

              <div>
                <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-2 block">
                  Command
                </label>
                <input
                  type="text"
                  value={newServerCmd}
                  onChange={(e) => setNewServerCmd(e.target.value)}
                  className="w-full bg-black border border-zinc-800 rounded-lg p-2.5 text-sm text-zinc-300 focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/50 transition-all"
                  placeholder="e.g. npx"
                />
              </div>

              <div>
                <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-2 block">
                  Arguments (comma separated)
                </label>
                <input
                  type="text"
                  value={newServerArgs}
                  onChange={(e) => setNewServerArgs(e.target.value)}
                  className="w-full bg-black border border-zinc-800 rounded-lg p-2.5 text-sm text-zinc-300 focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/50 transition-all"
                  placeholder="e.g. -y, @modelcontextprotocol/server-sqlite, test.db"
                />
              </div>

              {addServerError && (
                <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-md text-red-400 text-sm">
                  {addServerError}
                </div>
              )}
            </div>

            <div className="p-4 border-t border-zinc-800 bg-zinc-900/50 flex justify-end gap-3">
              <button 
                onClick={() => setSelectedDaemon(null)} 
                className="px-4 py-2 rounded-md text-sm font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleAddServer}
                disabled={isAddingServer}
                className="px-4 py-2 rounded-md text-sm font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-500/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
              >
                {isAddingServer ? (
                  <Activity className="w-4 h-4 animate-spin" />
                ) : (
                  <Plus className="w-4 h-4" />
                )}
                {isAddingServer ? 'Deploying...' : 'Deploy Server'}
              </button>
            </div>
          </div>
        </div>
      )}
      {/* Execution Modal */}
      {selectedTool && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-200">
          <div className="bg-[#111] border border-zinc-800 rounded-xl w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh] shadow-2xl">
            <div className="flex items-center justify-between p-4 border-b border-zinc-800 bg-zinc-900/50">
              <h2 className="font-semibold text-zinc-100 flex items-center gap-2">
                <Play className="w-4 h-4 text-blue-400" />
                Execute: <span className="text-emerald-400">{selectedTool.name}</span>
              </h2>
              <button 
                onClick={() => setSelectedTool(null)} 
                className="text-zinc-500 hover:text-zinc-300 transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            
            <div className="p-4 overflow-y-auto flex-1 space-y-6">
              {selectedTool.inputSchema && (
                <div>
                  <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-2 block">
                    Input Schema
                  </label>
                  <pre className="text-xs text-zinc-400 bg-black/50 p-3 rounded-lg border border-zinc-800/50 overflow-x-auto">
                    {JSON.stringify(selectedTool.inputSchema, null, 2)}
                  </pre>
                </div>
              )}
              
              <div>
                <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-2 block">
                  Arguments (JSON)
                </label>
                <textarea
                  value={toolArgs}
                  onChange={(e) => setToolArgs(e.target.value)}
                  className="w-full h-32 bg-black border border-zinc-800 rounded-lg p-3 text-sm text-zinc-300 font-mono focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/50 transition-all resize-y"
                  placeholder="{}"
                  spellCheck="false"
                />
              </div>

              {execResult && (
                <div className="animate-in fade-in slide-in-from-bottom-2">
                  <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-2 block">
                    Execution Result
                  </label>
                  <pre className="text-xs text-zinc-300 bg-black p-4 rounded-lg border border-zinc-800 overflow-x-auto max-h-64 whitespace-pre-wrap break-all">
                    {execResult}
                  </pre>
                </div>
              )}
            </div>

            <div className="p-4 border-t border-zinc-800 bg-zinc-900/50 flex justify-end gap-3">
              <button 
                onClick={() => setSelectedTool(null)} 
                className="px-4 py-2 rounded-md text-sm font-medium text-zinc-400 hover:text-zinc-200 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleExecute}
                disabled={isExecuting}
                className="px-4 py-2 rounded-md text-sm font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20 hover:bg-blue-500/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
              >
                {isExecuting ? (
                  <Activity className="w-4 h-4 animate-spin" />
                ) : (
                  <Terminal className="w-4 h-4" />
                )}
                {isExecuting ? 'Running...' : 'Run Tool'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
