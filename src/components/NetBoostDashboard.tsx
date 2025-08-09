// src/components/NetBoostDashboard.tsx
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Types matching the Rust backend
interface PerformanceStats {
  packets_received: number;
  packets_forwarded: number;
  packets_dropped: number;
  bytes_received: number;
  bytes_forwarded: number;
  average_latency: { secs: number; nanos: number };
  packet_loss_rate: number;
  bandwidth_usage: number;
  uptime: { secs: number; nanos: number };
}

interface ServiceStatus {
  is_running: boolean;
  uptime_seconds?: number;
  virtual_interface_name?: string;
}

interface PhysicalInterface {
  name: string;
  description: string;
  ip_address: string;
  index: number;
}

interface SystemInfo {
  os: string;
  arch: string;
  version: string;
  build_date: string;
}

const NetBoostDashboard: React.FC = () => {
  const [serviceStatus, setServiceStatus] = useState<ServiceStatus>({ is_running: false });
  const [performanceStats, setPerformanceStats] = useState<PerformanceStats | null>(null);
  const [interfaces, setInterfaces] = useState<PhysicalInterface[]>([]);
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [loadBalancingMode, setLoadBalancingMode] = useState<string>('balanced');
  const [isAggregationEnabled, setIsAggregationEnabled] = useState<boolean>(true);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const [error, setError] = useState<string>('');
  const [success, setSuccess] = useState<string>('');

  // Utility function to convert Rust Duration to milliseconds
  const durationToMs = (duration: { secs: number; nanos: number }): number => {
    return duration.secs * 1000 + duration.nanos / 1_000_000;
  };

  // Utility function to format bytes
  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  // Utility function to format uptime
  const formatUptime = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  // Clear messages after timeout
  useEffect(() => {
    if (error) {
      const timer = setTimeout(() => setError(''), 5000);
      return () => clearTimeout(timer);
    }
  }, [error]);

  useEffect(() => {
    if (success) {
      const timer = setTimeout(() => setSuccess(''), 3000);
      return () => clearTimeout(timer);
    }
  }, [success]);

  // Load initial data
  useEffect(() => {
    loadSystemInfo();
    loadNetworkInterfaces();
    updateServiceStatus();
  }, []);

  // Auto-refresh performance stats when service is running
  useEffect(() => {
    if (serviceStatus.is_running) {
      const interval = setInterval(() => {
        updatePerformanceStats();
        updateServiceStatus();
      }, 2000);
      return () => clearInterval(interval);
    }
  }, [serviceStatus.is_running]);

  const loadSystemInfo = async () => {
    try {
      const info = await invoke<SystemInfo>('get_system_info');
      setSystemInfo(info);
    } catch (err) {
      console.error('Failed to load system info:', err);
    }
  };

  const loadNetworkInterfaces = async () => {
    try {
      const interfaceList = await invoke<PhysicalInterface[]>('get_network_interfaces');
      setInterfaces(interfaceList);
    } catch (err) {
      setError('Failed to load network interfaces: ' + String(err));
    }
  };

  const updateServiceStatus = async () => {
    try {
      const status = await invoke<ServiceStatus>('get_service_status');
      setServiceStatus(status);
    } catch (err) {
      console.error('Failed to get service status:', err);
    }
  };

  const updatePerformanceStats = async () => {
    if (!serviceStatus.is_running) return;
    
    try {
      const stats = await invoke<PerformanceStats>('get_performance_stats');
      setPerformanceStats(stats);
    } catch (err) {
      console.error('Failed to get performance stats:', err);
    }
  };

  const startService = async () => {
    setIsStarting(true);
    setError('');
    try {
      const result = await invoke<string>('start_netboost');
      setSuccess(result);
      await updateServiceStatus();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsStarting(false);
    }
  };

  const stopService = async () => {
    setIsStopping(true);
    setError('');
    try {
      const result = await invoke<string>('stop_netboost');
      setSuccess(result);
      setPerformanceStats(null);
      await updateServiceStatus();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsStopping(false);
    }
  };

  const changeLoadBalancingMode = async (mode: string) => {
    try {
      const result = await invoke<string>('set_load_balancing_mode', { mode });
      setLoadBalancingMode(mode);
      setSuccess(result);
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900 text-white p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <header className="mb-8">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-4xl font-bold bg-gradient-to-r from-blue-400 to-purple-400 bg-clip-text text-transparent">
                NetBoost Pro
              </h1>
              <p className="text-slate-400 mt-2">Intelligent Network Optimization System</p>
            </div>
            <div className="text-right">
              {systemInfo && (
                <div className="text-sm text-slate-400">
                  <div>Version {systemInfo.version}</div>
                  <div>{systemInfo.os} ({systemInfo.arch})</div>
                </div>
              )}
            </div>
          </div>
        </header>

        {/* Status Messages */}
        {error && (
          <div className="mb-4 p-4 bg-red-500/20 border border-red-500/50 rounded-lg text-red-200">
            {error}
          </div>
        )}
        {success && (
          <div className="mb-4 p-4 bg-green-500/20 border border-green-500/50 rounded-lg text-green-200">
            {success}
          </div>
        )}

        {/* Service Control */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
          <div className="lg:col-span-2">
            <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-2xl font-semibold">Service Control</h2>
                <div className="flex items-center space-x-2">
                  <div className={`w-3 h-3 rounded-full ${serviceStatus.is_running ? 'bg-green-400 animate-pulse' : 'bg-red-400'}`}></div>
                  <span className={serviceStatus.is_running ? 'text-green-400' : 'text-red-400'}>
                    {serviceStatus.is_running ? 'Running' : 'Stopped'}
                  </span>
                </div>
              </div>
              
              <div className="flex space-x-4">
                <button
                  onClick={startService}
                  disabled={serviceStatus.is_running || isStarting}
                  className="px-6 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 disabled:cursor-not-allowed rounded-lg font-medium transition-colors"
                >
                  {isStarting ? 'Starting...' : 'Start NetBoost'}
                </button>
                <button
                  onClick={stopService}
                  disabled={!serviceStatus.is_running || isStopping}
                  className="px-6 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 disabled:cursor-not-allowed rounded-lg font-medium transition-colors"
                >
                  {isStopping ? 'Stopping...' : 'Stop NetBoost'}
                </button>
              </div>

              {serviceStatus.is_running && (
                <div className="mt-4 p-3 bg-slate-700/50 rounded-lg">
                  <div className="text-sm text-slate-400">Uptime</div>
                  <div className="font-mono text-green-400">{formatUptime(serviceStatus.uptime_seconds || 0)}</div>
                  {serviceStatus.virtual_interface_name && (
                    <>
                      <div className="text-sm text-slate-400 mt-2">Virtual Interface</div>
                      <div className="font-mono text-blue-400">{serviceStatus.virtual_interface_name}</div>
                    </>
                  )}
                </div>
              )}
            </div>
          </div>

          <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
            <h3 className="text-lg font-semibold mb-4">Load Balancing</h3>
            <select
              value={loadBalancingMode}
              onChange={(e) => changeLoadBalancingMode(e.target.value)}
              disabled={!serviceStatus.is_running}
              className="w-full p-2 bg-slate-700 border border-slate-600 rounded-lg text-white disabled:opacity-50"
            >
              <option value="balanced">Balanced</option>
              <option value="round_robin">Round Robin</option>
              <option value="latency_based">Latency Based</option>
              <option value="bandwidth_based">Bandwidth Based</option>
            </select>
          </div>
        </div>

        {/* Connection Aggregation Control */}
        <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6 mb-8">
          <h2 className="text-2xl font-semibold mb-4">Connection Aggregation</h2>
          <div className="flex items-center space-x-4">
            <p className="text-slate-400">Enable or disable connection aggregation.</p>
            <button
              onClick={() => {
                // This is a placeholder for now
                const new_state = !isAggregationEnabled;
                invoke('set_connection_aggregation', { enabled: new_state })
                  .then(() => {
                    setIsAggregationEnabled(new_state);
                    setSuccess(`Connection aggregation ${new_state ? 'enabled' : 'disabled'}`);
                  })
                  .catch(err => setError(String(err)));
              }}
              className={`px-6 py-2 rounded-lg font-medium transition-colors ${
                isAggregationEnabled
                  ? 'bg-blue-600 hover:bg-blue-700'
                  : 'bg-gray-600 hover:bg-gray-700'
              }`}
            >
              {isAggregationEnabled ? 'Enabled' : 'Disabled'}
            </button>
          </div>
        </div>

        {/* Performance Stats */}
        {performanceStats && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
              <div className="text-slate-400 text-sm">Packets Processed</div>
              <div className="text-2xl font-bold text-blue-400">
                {performanceStats.packets_forwarded.toLocaleString()}
              </div>
              <div className="text-sm text-slate-500">
                {performanceStats.packets_received.toLocaleString()} received
              </div>
            </div>

            <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
              <div className="text-slate-400 text-sm">Average Latency</div>
              <div className="text-2xl font-bold text-green-400">
                {durationToMs(performanceStats.average_latency).toFixed(1)}ms
              </div>
            </div>

            <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
              <div className="text-slate-400 text-sm">Bandwidth Usage</div>
              <div className="text-2xl font-bold text-purple-400">
                {formatBytes(performanceStats.bandwidth_usage)}/s
              </div>
            </div>

            <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
              <div className="text-slate-400 text-sm">Packet Loss</div>
              <div className={`text-2xl font-bold ${performanceStats.packet_loss_rate > 0.05 ? 'text-red-400' : 'text-green-400'}`}>
                {(performanceStats.packet_loss_rate * 100).toFixed(2)}%
              </div>
            </div>
          </div>
        )}

        {/* Network Interfaces */}
        <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
          <h3 className="text-xl font-semibold mb-4">Network Interfaces</h3>
          <div className="space-y-3">
            {interfaces.map((iface) => (
              <div key={iface.index} className="p-4 bg-slate-700/50 rounded-lg">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="font-medium text-blue-400">{iface.name}</div>
                    <div className="text-sm text-slate-400">{iface.description}</div>
                  </div>
                  <div className="text-right">
                    <div className="font-mono text-sm">{iface.ip_address}</div>
                    <div className="text-xs text-slate-500">Index: {iface.index}</div>
                  </div>
                </div>
                {/* Placeholder for per-interface stats */}
                <div className="mt-2 pt-2 border-t border-slate-600/50 text-xs text-slate-400">
                  Stats: (coming soon)
                </div>
              </div>
            ))}
            {interfaces.length === 0 && (
              <div className="text-slate-400 text-center py-4">
                No network interfaces detected
              </div>
            )}
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div className="bg-slate-800/50 backdrop-blur-sm border border-slate-700 rounded-xl p-6">
            <h3 className="text-xl font-semibold mb-4">System Information</h3>
            {performanceStats && (
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-slate-400">Uptime:</span>
                  <span>{formatUptime(performanceStats.uptime.secs)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Data Received:</span>
                  <span>{formatBytes(performanceStats.bytes_received)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Data Forwarded:</span>
                  <span>{formatBytes(performanceStats.bytes_forwarded)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-slate-400">Packets Dropped:</span>
                  <span className={performanceStats.packets_dropped > 0 ? 'text-red-400' : 'text-green-400'}>
                    {performanceStats.packets_dropped.toLocaleString()}
                  </span>
                </div>
              </div>
            )}
            {!performanceStats && serviceStatus.is_running && (
              <div className="text-slate-400 text-center py-4">
                Loading performance data...
              </div>
            )}
            {!serviceStatus.is_running && (
              <div className="text-slate-400 text-center py-4">
                Start NetBoost Pro to view performance data
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default NetBoostDashboard;