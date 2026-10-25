import React, { useState } from 'react';
import { Box, Key, Lock, Eye, EyeOff, AlertCircle, ArrowRight, ShieldCheck, Database } from 'lucide-react';
import { S3Api, setStoredCredentials, type Credentials } from '../api/s3Client';

interface LoginViewProps {
  onLoginSuccess: (creds: Credentials) => void;
}

export const LoginView: React.FC<LoginViewProps> = ({ onLoginSuccess }) => {
  const [accessKey, setAccessKey] = useState('');
  const [secretKey, setSecretKey] = useState('');
  const [showSecret, setShowSecret] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const handleLogin = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!accessKey.trim() || !secretKey.trim()) {
      setErrorMessage('Please enter both Access Key ID and Secret Access Key.');
      return;
    }

    setIsLoading(true);
    setErrorMessage(null);

    const creds: Credentials = {
      accessKey: accessKey.trim(),
      secretKey: secretKey.trim(),
    };

    const result = await S3Api.validateCredentials(creds);

    if (result.valid) {
      setStoredCredentials(creds);
      onLoginSuccess(creds);
    } else {
      setErrorMessage(result.error || 'Authentication failed. Please verify credentials.');
      setIsLoading(false);
    }
  };

  const handleQuickPreset = (ak: string, sk: string) => {
    setAccessKey(ak);
    setSecretKey(sk);
    setErrorMessage(null);
  };

  return (
    <div
      style={{
        minHeight: '100vh',
        width: '100vw',
        backgroundColor: 'var(--bg-base)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '24px',
        position: 'relative',
        overflow: 'hidden',
      }}
    >
      {/* Background subtle radial gradient */}
      <div
        style={{
          position: 'absolute',
          top: '20%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          width: '600px',
          height: '400px',
          background: 'radial-gradient(circle, rgba(246, 130, 31, 0.08) 0%, rgba(13, 15, 20, 0) 70%)',
          pointerEvents: 'none',
        }}
      />

      <div
        className="card animate-fade-in"
        style={{
          width: '100%',
          maxWidth: '440px',
          padding: '36px 32px',
          backgroundColor: 'var(--bg-surface)',
          border: '1px solid var(--border-subtle)',
          boxShadow: '0 20px 40px -15px rgba(0, 0, 0, 0.5), 0 0 1px 1px rgba(255, 255, 255, 0.05)',
          zIndex: 1,
        }}
      >
        {/* Brand Header */}
        <div style={{ textAlign: 'center', marginBottom: '28px' }}>
          <div
            style={{
              width: '44px',
              height: '44px',
              borderRadius: 'var(--radius-md)',
              backgroundColor: 'var(--cf-orange)',
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#ffffff',
              boxShadow: '0 4px 14px rgba(246, 130, 31, 0.35)',
              marginBottom: '16px',
            }}
          >
            <Box size={24} strokeWidth={2.5} />
          </div>

          <h1
            style={{
              fontSize: '1.25rem',
              fontWeight: '700',
              letterSpacing: '-0.02em',
              color: 'var(--text-primary)',
              margin: '0 0 6px 0',
            }}
          >
            nanoio
          </h1>
          <p
            style={{
              fontSize: '0.8125rem',
              color: 'var(--text-secondary)',
              margin: 0,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '6px',
            }}
          >
            <ShieldCheck size={14} color="var(--cf-orange)" />
            <span>Strict SigV4 & IAM Management Console</span>
          </p>
        </div>

        {/* Error Alert */}
        {errorMessage && (
          <div
            style={{
              backgroundColor: 'rgba(239, 68, 68, 0.1)',
              border: '1px solid rgba(239, 68, 68, 0.25)',
              borderRadius: 'var(--radius-sm)',
              padding: '10px 14px',
              marginBottom: '20px',
              display: 'flex',
              alignItems: 'flex-start',
              gap: '10px',
              color: 'var(--status-red)',
              fontSize: '0.8125rem',
              lineHeight: '1.4',
            }}
          >
            <AlertCircle size={16} style={{ flexShrink: 0, marginTop: '2px' }} />
            <span>{errorMessage}</span>
          </div>
        )}

        {/* Login Form */}
        <form onSubmit={handleLogin} style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          {/* Access Key ID */}
          <div>
            <label
              style={{
                display: 'block',
                fontSize: '0.75rem',
                fontWeight: '600',
                color: 'var(--text-secondary)',
                marginBottom: '6px',
                textTransform: 'uppercase',
                letterSpacing: '0.04em',
              }}
            >
              Access Key ID
            </label>
            <div style={{ position: 'relative' }}>
              <input
                type="text"
                className="input"
                placeholder="e.g. <YOUR_ACCESS_KEY>"
                value={accessKey}
                onChange={(e) => setAccessKey(e.target.value)}
                autoFocus
                disabled={isLoading}
                style={{
                  paddingLeft: '36px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '0.875rem',
                }}
              />
              <Key
                size={16}
                color="var(--text-muted)"
                style={{
                  position: 'absolute',
                  left: '12px',
                  top: '50%',
                  transform: 'translateY(-50%)',
                }}
              />
            </div>
          </div>

          {/* Secret Access Key */}
          <div>
            <label
              style={{
                display: 'block',
                fontSize: '0.75rem',
                fontWeight: '600',
                color: 'var(--text-secondary)',
                marginBottom: '6px',
                textTransform: 'uppercase',
                letterSpacing: '0.04em',
              }}
            >
              Secret Access Key
            </label>
            <div style={{ position: 'relative' }}>
              <input
                type={showSecret ? 'text' : 'password'}
                className="input"
                placeholder="••••••••••••••••"
                value={secretKey}
                onChange={(e) => setSecretKey(e.target.value)}
                disabled={isLoading}
                style={{
                  paddingLeft: '36px',
                  paddingRight: '36px',
                  fontFamily: 'var(--font-mono)',
                  fontSize: '0.875rem',
                }}
              />
              <Lock
                size={16}
                color="var(--text-muted)"
                style={{
                  position: 'absolute',
                  left: '12px',
                  top: '50%',
                  transform: 'translateY(-50%)',
                }}
              />
              <button
                type="button"
                onClick={() => setShowSecret(!showSecret)}
                style={{
                  position: 'absolute',
                  right: '10px',
                  top: '50%',
                  transform: 'translateY(-50%)',
                  background: 'none',
                  border: 'none',
                  color: 'var(--text-muted)',
                  cursor: 'pointer',
                  padding: '4px',
                  display: 'flex',
                }}
                tabIndex={-1}
              >
                {showSecret ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>
            </div>
          </div>

          {/* Submit Button */}
          <button
            type="submit"
            className="btn btn-primary"
            disabled={isLoading}
            style={{
              height: '42px',
              marginTop: '8px',
              fontSize: '0.875rem',
              fontWeight: '600',
              justifyContent: 'center',
            }}
          >
            {isLoading ? (
              <>
                <span className="animate-spin">⏳</span>
                <span>Verifying SigV4 Signature...</span>
              </>
            ) : (
              <>
                <span>Sign In to Console</span>
                <ArrowRight size={16} />
              </>
            )}
          </button>
        </form>

        {/* Quick Credentials Presets */}
        <div style={{ marginTop: '28px', paddingTop: '20px', borderTop: '1px solid var(--border-subtle)' }}>
          <div
            style={{
              fontSize: '0.6875rem',
              color: 'var(--text-muted)',
              marginBottom: '10px',
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
              fontWeight: '600',
            }}
          >
            Quick Fill IAM Credentials
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => handleQuickPreset('nanoioadmin', 'nanoioadmin')}
              style={{
                justifyContent: 'space-between',
                padding: '8px 12px',
                fontSize: '0.75rem',
                height: 'auto',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Database size={13} color="var(--cf-orange)" />
                <span style={{ fontWeight: '600', color: 'var(--text-primary)' }}>nanoioadmin</span>
                <span style={{ color: 'var(--text-muted)', fontSize: '0.7rem' }}>(Root Admin)</span>
              </div>
              <span className="badge badge-green" style={{ fontSize: '0.65rem' }}>Full Access</span>
            </button>

            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => handleQuickPreset('developer', 'dev123')}
              style={{
                justifyContent: 'space-between',
                padding: '8px 12px',
                fontSize: '0.75rem',
                height: 'auto',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Key size={13} color="var(--cf-blue)" />
                <span style={{ fontWeight: '600', color: 'var(--text-primary)' }}>developer</span>
                <span style={{ color: 'var(--text-muted)', fontSize: '0.7rem' }}>(IAM User)</span>
              </div>
              <span className="badge badge-blue" style={{ fontSize: '0.65rem' }}>ReadWrite</span>
            </button>

            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => handleQuickPreset('reader', 'reader123')}
              style={{
                justifyContent: 'space-between',
                padding: '8px 12px',
                fontSize: '0.75rem',
                height: 'auto',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <Key size={13} color="var(--text-secondary)" />
                <span style={{ fontWeight: '600', color: 'var(--text-primary)' }}>reader</span>
                <span style={{ color: 'var(--text-muted)', fontSize: '0.7rem' }}>(Audit User)</span>
              </div>
              <span className="badge badge-gray" style={{ fontSize: '0.65rem' }}>ReadOnly</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
