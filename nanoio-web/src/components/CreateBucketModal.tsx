import React, { useState } from 'react';
import { X, CheckCircle2, AlertCircle } from 'lucide-react';

interface CreateBucketModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string) => Promise<void>;
}

export const CreateBucketModal: React.FC<CreateBucketModalProps> = ({
  isOpen,
  onClose,
  onCreate,
}) => {
  const [bucketName, setBucketName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const validateName = (name: string): string | null => {
    if (name.length < 3 || name.length > 63) {
      return 'Bucket name must be between 3 and 63 characters.';
    }
    if (!/^[a-z0-9.-]+$/.test(name)) {
      return 'Can only contain lowercase letters, numbers, hyphens, and periods.';
    }
    if (!/^[a-z0-9].*[a-z0-9]$/.test(name)) {
      return 'Must begin and end with a letter or number.';
    }
    if (name.includes('..') || name.includes('.-') || name.includes('-.')) {
      return 'Cannot contain consecutive formatting characters (.., .-, -.).';
    }
    return null;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const validationError = validateName(bucketName);
    if (validationError) {
      setError(validationError);
      return;
    }

    try {
      setIsSubmitting(true);
      setError(null);
      await onCreate(bucketName);
      setBucketName('');
      onClose();
    } catch (err: any) {
      setError(err.message || 'Failed to create bucket');
    } finally {
      setIsSubmitting(false);
    }
  };

  const currentError = bucketName ? validateName(bucketName) : null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.65)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: '440px',
          backgroundColor: 'var(--bg-surface)',
          border: '1px solid var(--border-strong)',
          borderRadius: 'var(--radius-md)',
          boxShadow: 'var(--shadow-modal)',
          overflow: 'hidden',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            padding: '14px 18px',
            borderBottom: '1px solid var(--border-subtle)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span style={{ fontSize: '0.875rem', fontWeight: '600', color: '#ffffff' }}>
            Create Bucket
          </span>
          <button onClick={onClose} className="btn-icon">
            <X size={15} />
          </button>
        </div>

        <form onSubmit={handleSubmit} style={{ padding: '18px' }}>
          <div style={{ marginBottom: '14px' }}>
            <label style={{ display: 'block', fontSize: '0.75rem', fontWeight: '500', color: 'var(--text-secondary)', marginBottom: '6px' }}>
              Bucket name
            </label>
            <input
              type="text"
              placeholder="e.g. production-assets"
              value={bucketName}
              onChange={(e) => {
                setBucketName(e.target.value.toLowerCase().trim());
                setError(null);
              }}
              autoFocus
              className="input-field"
              style={{ fontFamily: 'var(--font-mono)' }}
            />

            {currentError && (
              <div style={{ fontSize: '0.6875rem', color: '#f87171', marginTop: '6px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                <AlertCircle size={11} />
                <span>{currentError}</span>
              </div>
            )}
            {!currentError && bucketName.length >= 3 && (
              <div style={{ fontSize: '0.6875rem', color: 'var(--status-green)', marginTop: '6px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                <CheckCircle2 size={11} />
                <span>Bucket name is valid.</span>
              </div>
            )}
          </div>

          {error && (
            <div style={{ padding: '8px 12px', borderRadius: 'var(--radius-xs)', backgroundColor: 'var(--status-red-bg)', color: '#f87171', fontSize: '0.75rem', marginBottom: '14px' }}>
              {error}
            </div>
          )}

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px', marginTop: '20px' }}>
            <button type="button" onClick={onClose} className="btn btn-secondary">
              Cancel
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !bucketName || !!currentError}
              className="btn btn-primary"
              style={{ opacity: !bucketName || !!currentError ? 0.4 : 1 }}
            >
              {isSubmitting ? 'Creating...' : 'Create Bucket'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
