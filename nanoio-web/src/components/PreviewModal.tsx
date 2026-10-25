import React, { useEffect, useState } from 'react';
import { X, Download, Copy, Check } from 'lucide-react';
import { S3Api } from '../api/s3Client';
import type { S3Object } from '../api/s3Client';
import { formatBytes, formatDate } from '../utils/format';

interface PreviewModalProps {
  bucket: string;
  object: S3Object | null;
  onClose: () => void;
}

export const PreviewModal: React.FC<PreviewModalProps> = ({
  bucket,
  object,
  onClose,
}) => {
  const [copied, setCopied] = useState(false);
  const [textContent, setTextContent] = useState<string | null>(null);
  const [loadingText, setLoadingText] = useState(false);

  if (!object) return null;

  const url = S3Api.getObjectUrl(bucket, object.key);
  const ext = object.key.split('.').pop()?.toLowerCase() || '';

  const isImage = ['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp'].includes(ext);
  const isVideo = ['mp4', 'webm', 'ogg'].includes(ext);
  const isText = ['txt', 'json', 'md', 'xml', 'html', 'css', 'js', 'ts', 'yaml', 'yml', 'rs'].includes(ext);

  useEffect(() => {
    if (isText && object.size < 500 * 1024) {
      setLoadingText(true);
      fetch(url)
        .then((res) => res.text())
        .then((t) => setTextContent(t))
        .catch(() => setTextContent('(Failed to fetch preview content)'))
        .finally(() => setLoadingText(false));
    } else {
      setTextContent(null);
    }
  }, [url, isText, object.size]);

  const handleCopyUrl = () => {
    navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
        padding: '20px',
      }}
      onClick={onClose}
    >
      <div
        style={{
          width: '760px',
          maxHeight: '85vh',
          display: 'flex',
          flexDirection: 'column',
          backgroundColor: 'var(--bg-surface)',
          border: '1px solid var(--border-strong)',
          borderRadius: 'var(--radius-md)',
          boxShadow: 'var(--shadow-modal)',
          overflow: 'hidden',
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Modal Header */}
        <div
          style={{
            padding: '12px 18px',
            borderBottom: '1px solid var(--border-subtle)',
            backgroundColor: 'var(--bg-surface-raised)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <div>
            <div style={{ fontSize: '0.875rem', fontWeight: '600', color: '#ffffff', fontFamily: 'var(--font-mono)' }}>
              {object.key}
            </div>
            <div style={{ fontSize: '0.6875rem', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', marginTop: '2px' }}>
              Size: {formatBytes(object.size)} • Modified: {formatDate(object.lastModified)} • ETag: {object.etag}
            </div>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <button onClick={handleCopyUrl} className="btn btn-secondary" style={{ height: '28px', padding: '0 8px', fontSize: '0.75rem' }}>
              {copied ? <Check size={12} color="var(--status-green)" /> : <Copy size={12} />}
              <span>{copied ? 'Copied' : 'Copy URL'}</span>
            </button>
            <a href={url} download={object.key.split('/').pop()} className="btn btn-primary" style={{ height: '28px', padding: '0 10px', fontSize: '0.75rem', textDecoration: 'none' }}>
              <Download size={12} />
              <span>Download</span>
            </a>
            <button onClick={onClose} className="btn-icon">
              <X size={15} />
            </button>
          </div>
        </div>

        {/* Content Area */}
        <div style={{ padding: '20px', overflowY: 'auto', flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', backgroundColor: '#090b0e', minHeight: '260px' }}>
          {isImage ? (
            <img
              src={url}
              alt={object.key}
              style={{ maxWidth: '100%', maxHeight: '55vh', borderRadius: 'var(--radius-sm)', objectFit: 'contain' }}
            />
          ) : isVideo ? (
            <video
              src={url}
              controls
              style={{ maxWidth: '100%', maxHeight: '55vh', borderRadius: 'var(--radius-sm)' }}
            />
          ) : isText ? (
            loadingText ? (
              <div style={{ color: 'var(--text-muted)', fontSize: '0.8125rem' }}>Loading preview...</div>
            ) : (
              <pre
                style={{
                  width: '100%',
                  maxHeight: '52vh',
                  padding: '14px',
                  borderRadius: 'var(--radius-sm)',
                  backgroundColor: 'var(--bg-input)',
                  border: '1px solid var(--border-subtle)',
                  color: '#e5e7eb',
                  fontSize: '0.8125rem',
                  fontFamily: 'var(--font-mono)',
                  overflowX: 'auto',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-all',
                }}
              >
                {textContent}
              </pre>
            )
          ) : (
            <div style={{ textAlign: 'center', color: 'var(--text-muted)' }}>
              <p style={{ fontSize: '0.8125rem', marginBottom: '6px' }}>No inline preview available for this file type.</p>
              <p style={{ fontSize: '0.75rem' }}>Click Download above to view the file locally.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
