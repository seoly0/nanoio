import React, { useState, useRef } from 'react';
import {
  Upload,
  FolderPlus,
  Folder,
  FileText,
  FileImage,
  FileCode,
  File as FileGeneric,
  Trash2,
  Eye,
  Download,
  ChevronRight,
  CornerLeftUp,
  Inbox,
  Copy,
  Check,
} from 'lucide-react';
import { S3Api } from '../api/s3Client';
import type { S3Object } from '../api/s3Client';
import { formatBytes, formatDate } from '../utils/format';

interface ObjectExplorerProps {
  bucket: string;
  currentPrefix: string;
  objects: S3Object[];
  commonPrefixes: string[];
  isLoading: boolean;
  onNavigate: (prefix: string) => void;
  onUpload: (file: File) => Promise<void>;
  onDelete: (key: string) => Promise<void>;
  onPreview: (obj: S3Object) => void;
}

export const ObjectExplorer: React.FC<ObjectExplorerProps> = ({
  bucket,
  currentPrefix,
  objects,
  commonPrefixes,
  isLoading,
  onNavigate,
  onUpload,
  onDelete,
  onPreview,
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [showFolderInput, setShowFolderInput] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const pathSegments = currentPrefix.split('/').filter(Boolean);

  const getFileIcon = (filename: string) => {
    const ext = filename.split('.').pop()?.toLowerCase();
    if (['png', 'jpg', 'jpeg', 'gif', 'svg', 'webp'].includes(ext || '')) {
      return <FileImage size={15} color="#38bdf8" />;
    }
    if (['js', 'ts', 'jsx', 'tsx', 'html', 'css', 'json', 'xml', 'rs'].includes(ext || '')) {
      return <FileCode size={15} color="var(--cf-orange)" />;
    }
    if (['txt', 'md', 'pdf'].includes(ext || '')) {
      return <FileText size={15} color="#a78bfa" />;
    }
    return <FileGeneric size={15} color="var(--text-muted)" />;
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      for (let i = 0; i < e.dataTransfer.files.length; i++) {
        await onUpload(e.dataTransfer.files[i]);
      }
    }
  };

  const handleFileInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      for (let i = 0; i < e.target.files.length; i++) {
        await onUpload(e.target.files[i]);
      }
    }
  };

  const handleCreateFolder = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanName = newFolderName.trim().replace(/\//g, '');
    if (!cleanName) return;

    const folderKey = `${currentPrefix}${cleanName}/`;
    const emptyBlob = new Blob([], { type: 'application/x-directory' });
    await S3Api.uploadObject(bucket, folderKey, emptyBlob);
    setNewFolderName('');
    setShowFolderInput(false);
    onNavigate(currentPrefix);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(text);
    setTimeout(() => setCopiedKey(null), 1500);
  };

  const parentPrefix = pathSegments.length > 0
    ? pathSegments.slice(0, -1).join('/') + (pathSegments.length > 1 ? '/' : '')
    : null;

  return (
    <main
      style={{
        flex: 1,
        backgroundColor: 'var(--bg-app)',
        display: 'flex',
        flexDirection: 'column',
        height: 'calc(100vh - 52px)',
        overflow: 'hidden',
      }}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Bucket Sub-header */}
      <div
        style={{
          padding: '16px 24px',
          borderBottom: '1px solid var(--border-subtle)',
          backgroundColor: 'var(--bg-surface)',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}
      >
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <h2 style={{ fontSize: '1.125rem', fontWeight: '700', fontFamily: 'var(--font-mono)', color: '#ffffff' }}>
              {bucket}
            </h2>
            <span className="badge badge-orange">STANDARD</span>
            <span className="badge badge-gray">PATH-STYLE</span>
          </div>
          <div style={{ fontSize: '0.75rem', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', marginTop: '4px' }}>
            s3://{bucket}/{currentPrefix}
          </div>
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <input
            type="file"
            ref={fileInputRef}
            onChange={handleFileInputChange}
            multiple
            style={{ display: 'none' }}
          />
          <button
            onClick={() => setShowFolderInput(!showFolderInput)}
            className="btn btn-secondary"
          >
            <FolderPlus size={14} />
            <span>Create Folder</span>
          </button>
          <button
            onClick={() => fileInputRef.current?.click()}
            className="btn btn-primary"
          >
            <Upload size={14} />
            <span>Upload Object</span>
          </button>
        </div>
      </div>

      {/* Breadcrumb Path Bar */}
      <div
        style={{
          padding: '10px 24px',
          borderBottom: '1px solid var(--border-subtle)',
          backgroundColor: 'rgba(20, 23, 31, 0.6)',
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          fontSize: '0.8125rem',
          fontFamily: 'var(--font-mono)',
        }}
      >
        <span
          onClick={() => onNavigate('')}
          style={{
            cursor: 'pointer',
            color: pathSegments.length === 0 ? 'var(--cf-orange)' : 'var(--text-secondary)',
            fontWeight: pathSegments.length === 0 ? '600' : '400',
          }}
        >
          root/
        </span>
        {pathSegments.map((seg, idx) => {
          const segPrefix = pathSegments.slice(0, idx + 1).join('/') + '/';
          const isLast = idx === pathSegments.length - 1;
          return (
            <React.Fragment key={segPrefix}>
              <ChevronRight size={12} color="var(--text-muted)" />
              <span
                onClick={() => onNavigate(segPrefix)}
                style={{
                  cursor: 'pointer',
                  color: isLast ? 'var(--cf-orange)' : 'var(--text-secondary)',
                  fontWeight: isLast ? '600' : '400',
                }}
              >
                {seg}/
              </span>
            </React.Fragment>
          );
        })}
      </div>

      {/* New Folder Form */}
      {showFolderInput && (
        <form
          onSubmit={handleCreateFolder}
          style={{
            padding: '12px 24px',
            backgroundColor: 'var(--bg-surface-raised)',
            borderBottom: '1px solid var(--border-subtle)',
            display: 'flex',
            gap: '8px',
            alignItems: 'center',
          }}
        >
          <span style={{ fontSize: '0.8125rem', color: 'var(--text-secondary)' }}>Folder name:</span>
          <input
            type="text"
            placeholder="e.g. logs"
            value={newFolderName}
            onChange={(e) => setNewFolderName(e.target.value)}
            autoFocus
            className="input-field"
            style={{ maxWidth: '240px', height: '28px' }}
          />
          <button type="submit" className="btn btn-primary" style={{ height: '28px', padding: '0 10px' }}>
            Save
          </button>
          <button
            type="button"
            onClick={() => setShowFolderInput(false)}
            className="btn btn-secondary"
            style={{ height: '28px', padding: '0 10px' }}
          >
            Cancel
          </button>
        </form>
      )}

      {/* Drag & Drop Overlay */}
      {isDragging && (
        <div
          style={{
            margin: '16px 24px',
            padding: '24px',
            border: '1px dashed var(--cf-orange)',
            borderRadius: 'var(--radius-sm)',
            backgroundColor: 'var(--cf-orange-subtle)',
            textAlign: 'center',
            color: 'var(--cf-orange)',
          }}
        >
          <Upload size={24} style={{ margin: '0 auto 6px' }} />
          <div style={{ fontSize: '0.8125rem', fontWeight: '600' }}>Drop files to upload into current directory</div>
        </div>
      )}

      {/* Main Dense Table */}
      <div style={{ flex: 1, overflowY: 'auto' }}>
        <table className="data-table">
          <thead>
            <tr>
              <th style={{ width: '45%' }}>Object Name</th>
              <th style={{ width: '12%' }}>Storage Class</th>
              <th style={{ width: '12%' }}>Size</th>
              <th style={{ width: '18%' }}>Last Modified</th>
              <th style={{ width: '13%', textAlign: 'right' }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {/* Go Up Directory */}
            {parentPrefix !== null && (
              <tr
                onClick={() => onNavigate(parentPrefix)}
                style={{ cursor: 'pointer' }}
              >
                <td colSpan={5} style={{ color: 'var(--cf-orange)', fontFamily: 'var(--font-mono)' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                    <CornerLeftUp size={13} />
                    <span>..</span>
                  </div>
                </td>
              </tr>
            )}

            {/* Folders */}
            {commonPrefixes.map((prefix) => {
              const folderName = prefix.slice(currentPrefix.length).replace(/\/$/, '');
              return (
                <tr
                  key={prefix}
                  onClick={() => onNavigate(prefix)}
                  style={{ cursor: 'pointer' }}
                >
                  <td>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontFamily: 'var(--font-mono)' }}>
                      <Folder size={14} color="var(--cf-orange)" />
                      <span style={{ fontWeight: '600', color: '#ffffff' }}>{folderName}/</span>
                    </div>
                  </td>
                  <td style={{ color: 'var(--text-muted)' }}>-</td>
                  <td style={{ color: 'var(--text-muted)' }}>-</td>
                  <td style={{ color: 'var(--text-muted)' }}>-</td>
                  <td style={{ textAlign: 'right' }}></td>
                </tr>
              );
            })}

            {/* Objects */}
            {objects
              .filter((o) => !o.key.endsWith('/') || o.key !== currentPrefix)
              .map((obj) => {
                const fileName = obj.key.slice(currentPrefix.length);
                const downloadUrl = S3Api.getObjectUrl(bucket, obj.key);
                const isCopied = copiedKey === obj.key;

                return (
                  <tr key={obj.key}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        {getFileIcon(fileName)}
                        <span
                          onClick={() => onPreview(obj)}
                          style={{
                            cursor: 'pointer',
                            color: 'var(--text-primary)',
                            fontFamily: 'var(--font-mono)',
                            fontWeight: '500',
                          }}
                          onMouseEnter={(e) => (e.currentTarget.style.color = 'var(--cf-orange)')}
                          onMouseLeave={(e) => (e.currentTarget.style.color = 'var(--text-primary)')}
                        >
                          {fileName}
                        </span>
                      </div>
                    </td>
                    <td>
                      <span className="badge badge-gray">{obj.storageClass}</span>
                    </td>
                    <td style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)' }}>
                      {formatBytes(obj.size)}
                    </td>
                    <td style={{ color: 'var(--text-muted)', fontSize: '0.75rem' }}>
                      {formatDate(obj.lastModified)}
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'inline-flex', gap: '2px' }}>
                        <button
                          onClick={() => copyToClipboard(downloadUrl)}
                          className="btn-icon"
                          title="Copy URL"
                        >
                          {isCopied ? <Check size={13} color="var(--status-green)" /> : <Copy size={13} />}
                        </button>
                        <button
                          onClick={() => onPreview(obj)}
                          className="btn-icon"
                          title="Preview"
                        >
                          <Eye size={13} />
                        </button>
                        <a
                          href={downloadUrl}
                          download={fileName}
                          className="btn-icon"
                          title="Download"
                        >
                          <Download size={13} />
                        </a>
                        <button
                          onClick={() => {
                            if (confirm(`'${fileName}' 오브젝트를 삭제하시겠습니까?`)) {
                              onDelete(obj.key);
                            }
                          }}
                          className="btn-icon btn-danger"
                          title="Delete"
                        >
                          <Trash2 size={13} />
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}

            {/* Empty Directory */}
            {!isLoading && commonPrefixes.length === 0 && objects.length === 0 && (
              <tr>
                <td colSpan={5} style={{ textAlign: 'center', padding: '64px 20px', color: 'var(--text-muted)' }}>
                  <Inbox size={32} style={{ margin: '0 auto 10px', opacity: 0.3 }} />
                  <div style={{ fontSize: '0.875rem', fontWeight: '500', color: 'var(--text-secondary)' }}>
                    No objects in this directory
                  </div>
                  <div style={{ fontSize: '0.75rem', marginTop: '4px' }}>
                    Upload files using the button above or drag and drop files here.
                  </div>
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </main>
  );
};
