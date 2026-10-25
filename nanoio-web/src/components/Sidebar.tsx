import React, { useState } from 'react';
import { Plus, Search, Database, Trash2 } from 'lucide-react';
import type { S3Bucket } from '../api/s3Client';
import { formatDate } from '../utils/format';

interface SidebarProps {
  buckets: S3Bucket[];
  selectedBucket: string | null;
  onSelectBucket: (name: string) => void;
  onOpenCreateModal: () => void;
  onDeleteBucket: (name: string) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  buckets,
  selectedBucket,
  onSelectBucket,
  onOpenCreateModal,
  onDeleteBucket,
}) => {
  const [search, setSearch] = useState('');

  const filtered = buckets.filter((b) =>
    b.name.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <aside
      style={{
        width: '260px',
        backgroundColor: 'var(--bg-surface)',
        borderRight: '1px solid var(--border-subtle)',
        display: 'flex',
        flexDirection: 'column',
        height: 'calc(100vh - 52px)',
      }}
    >
      {/* Search & Actions Header */}
      <div style={{ padding: '14px 16px', borderBottom: '1px solid var(--border-subtle)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
          <span style={{ fontSize: '0.6875rem', fontWeight: '700', color: 'var(--text-muted)', letterSpacing: '0.05em', textTransform: 'uppercase' }}>
            Buckets ({buckets.length})
          </span>
          <button
            onClick={onOpenCreateModal}
            className="btn btn-primary"
            style={{ height: '26px', padding: '0 8px', fontSize: '0.75rem' }}
          >
            <Plus size={13} strokeWidth={2.5} />
            <span>Create</span>
          </button>
        </div>

        <div style={{ position: 'relative' }}>
          <Search
            size={13}
            color="var(--text-muted)"
            style={{ position: 'absolute', left: '9px', top: '50%', transform: 'translateY(-50%)' }}
          />
          <input
            type="text"
            placeholder="Search buckets..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="input-field"
            style={{ paddingLeft: '28px', height: '28px', fontSize: '0.75rem' }}
          />
        </div>
      </div>

      {/* Bucket List */}
      <div style={{ flex: 1, overflowY: 'auto', padding: '8px' }}>
        {filtered.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '32px 12px', color: 'var(--text-muted)', fontSize: '0.75rem' }}>
            {buckets.length === 0 ? 'No buckets created.' : 'No matches found.'}
          </div>
        ) : (
          filtered.map((b) => {
            const isSelected = selectedBucket === b.name;
            return (
              <div
                key={b.name}
                onClick={() => onSelectBucket(b.name)}
                style={{
                  padding: '8px 10px',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: isSelected ? 'var(--bg-surface-raised)' : 'transparent',
                  borderLeft: isSelected ? '3px solid var(--cf-orange)' : '3px solid transparent',
                  cursor: 'pointer',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  marginBottom: '2px',
                  transition: 'background-color 0.1s ease',
                }}
                onMouseEnter={(e) => {
                  if (!isSelected) e.currentTarget.style.backgroundColor = 'var(--bg-surface-hover)';
                }}
                onMouseLeave={(e) => {
                  if (!isSelected) e.currentTarget.style.backgroundColor = 'transparent';
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px', overflow: 'hidden' }}>
                  <Database
                    size={14}
                    color={isSelected ? 'var(--cf-orange)' : 'var(--text-muted)'}
                  />
                  <div style={{ overflow: 'hidden' }}>
                    <div
                      style={{
                        fontSize: '0.8125rem',
                        fontWeight: isSelected ? '600' : '400',
                        color: isSelected ? '#ffffff' : 'var(--text-primary)',
                        fontFamily: 'var(--font-mono)',
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                      }}
                    >
                      {b.name}
                    </div>
                    <div style={{ fontSize: '0.6875rem', color: 'var(--text-muted)', marginTop: '1px' }}>
                      {formatDate(b.creationDate).split(' ')[0]}
                    </div>
                  </div>
                </div>

                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (confirm(`Bucket '${b.name}'를 삭제하시겠습니까? (버킷 내부가 비어있어야 합니다)`)) {
                      onDeleteBucket(b.name);
                    }
                  }}
                  className="btn-icon"
                  style={{ opacity: 0.6 }}
                  title="버킷 삭제"
                >
                  <Trash2 size={13} />
                </button>
              </div>
            );
          })
        )}
      </div>
    </aside>
  );
};
