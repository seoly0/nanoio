import React from 'react';
import { Box, RefreshCw, Terminal, Cpu, User, LogOut } from 'lucide-react';
import { formatBytes } from '../utils/format';

interface HeaderProps {
  bucketCount: number;
  totalObjects: number;
  totalSize: number;
  isRefreshing: boolean;
  currentAccessKey: string;
  onRefresh: () => void;
  onLogout: () => void;
}

export const Header: React.FC<HeaderProps> = ({
  bucketCount,
  totalObjects,
  totalSize,
  isRefreshing,
  currentAccessKey,
  onRefresh,
  onLogout,
}) => {
  return (
    <header
      style={{
        height: '52px',
        borderBottom: '1px solid var(--border-subtle)',
        backgroundColor: 'var(--bg-surface)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 20px',
        userSelect: 'none',
      }}
    >
      {/* Brand & Region */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <div
            style={{
              width: '26px',
              height: '26px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--cf-orange)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#ffffff',
            }}
          >
            <Box size={16} strokeWidth={2.5} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span style={{ fontSize: '0.875rem', fontWeight: '700', letterSpacing: '-0.01em', color: '#ffffff' }}>
              nanoio
            </span>
            <span style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>/</span>
            <span style={{ fontSize: '0.8125rem', color: 'var(--text-secondary)', fontWeight: '500' }}>
              R2 Compatible
            </span>
          </div>
        </div>

        <div style={{ height: '16px', width: '1px', backgroundColor: 'var(--border-subtle)' }} />

        {/* Live Status Pill */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
          <span className="status-dot"></span>
          <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', fontFamily: 'var(--font-mono)' }}>
            localhost:9000
          </span>
          <span className="badge badge-green" style={{ marginLeft: '4px' }}>
            READY
          </span>
        </div>
      </div>

      {/* Cluster Telemetry & Quick Action */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '16px', fontFamily: 'var(--font-mono)', fontSize: '0.75rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: 'var(--text-secondary)' }}>
            <Cpu size={13} color="var(--cf-orange)" />
            <span>BUCKETS:</span>
            <strong style={{ color: 'var(--text-primary)' }}>{bucketCount}</strong>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: 'var(--text-secondary)' }}>
            <Terminal size={13} color="var(--cf-orange)" />
            <span>OBJECTS:</span>
            <strong style={{ color: 'var(--text-primary)' }}>{totalObjects}</strong>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px', color: 'var(--text-secondary)' }}>
            <span>STORAGE:</span>
            <strong style={{ color: 'var(--text-primary)' }}>{formatBytes(totalSize)}</strong>
          </div>
        </div>

        <button
          onClick={onRefresh}
          className="btn btn-secondary"
          style={{ height: '30px', padding: '0 10px', fontSize: '0.75rem' }}
          title="클러스터 상태 동기화"
        >
          <RefreshCw size={12} className={isRefreshing ? 'animate-spin' : ''} />
          <span>Sync</span>
        </button>

        <div style={{ height: '16px', width: '1px', backgroundColor: 'var(--border-subtle)' }} />

        {/* Current IAM User Badge & Logout */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '4px 10px',
              borderRadius: 'var(--radius-sm)',
              backgroundColor: 'var(--bg-elevated)',
              border: '1px solid var(--border-subtle)',
              fontSize: '0.75rem',
              fontFamily: 'var(--font-mono)',
              color: 'var(--text-primary)',
            }}
            title="현재 로그인된 IAM 계정"
          >
            <User size={13} color="var(--cf-orange)" />
            <span style={{ fontWeight: '600' }}>{currentAccessKey}</span>
          </div>

          <button
            onClick={onLogout}
            className="btn btn-secondary"
            style={{
              height: '30px',
              padding: '0 10px',
              fontSize: '0.75rem',
              color: 'var(--text-secondary)',
            }}
            title="콘솔 로그아웃"
          >
            <LogOut size={12} />
            <span>Sign Out</span>
          </button>
        </div>
      </div>
    </header>
  );
};
