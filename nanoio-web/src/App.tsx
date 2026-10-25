import { useState, useEffect, useCallback } from 'react';
import { S3Api, getStoredCredentials, clearStoredCredentials, type Credentials } from './api/s3Client';
import type { S3Bucket, S3Object } from './api/s3Client';
import { Header } from './components/Header';
import { Sidebar } from './components/Sidebar';
import { ObjectExplorer } from './components/ObjectExplorer';
import { CreateBucketModal } from './components/CreateBucketModal';
import { PreviewModal } from './components/PreviewModal';
import { LoginView } from './components/LoginView';
import { Database, AlertCircle } from 'lucide-react';

export function App() {
  const [credentials, setCredentials] = useState<Credentials | null>(() => getStoredCredentials());
  const [buckets, setBuckets] = useState<S3Bucket[]>([]);
  const [selectedBucket, setSelectedBucket] = useState<string | null>(null);
  const [currentPrefix, setCurrentPrefix] = useState<string>('');
  const [objects, setObjects] = useState<S3Object[]>([]);
  const [commonPrefixes, setCommonPrefixes] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [isRefreshing, setIsRefreshing] = useState<boolean>(false);
  const [serverError, setServerError] = useState<string | null>(null);

  // 모달 상태
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [previewObject, setPreviewObject] = useState<S3Object | null>(null);

  // 통계
  const [totalObjectsCount, setTotalObjectsCount] = useState(0);
  const [totalStorageSize, setTotalStorageSize] = useState(0);

  // 1. 버킷 목록 로드
  const loadBuckets = useCallback(async () => {
    if (!credentials) return;
    try {
      setIsRefreshing(true);
      setServerError(null);
      const bList = await S3Api.listBuckets();
      setBuckets(bList);
      if (bList.length > 0 && !selectedBucket) {
        setSelectedBucket(bList[0].name);
      }
    } catch (err: any) {
      if (err.message?.includes('403') || err.message?.includes('AccessDenied')) {
        setServerError('Authentication failed or action denied. Please sign in again.');
      } else {
        setServerError('Cannot connect to S3 Endpoint at http://localhost:9000. Ensure the storage server is running.');
      }
    } finally {
      setIsRefreshing(false);
    }
  }, [credentials, selectedBucket]);

  useEffect(() => {
    if (credentials) {
      loadBuckets();
    }
  }, [credentials, loadBuckets]);

  const handleLoginSuccess = (creds: Credentials) => {
    setCredentials(creds);
    setServerError(null);
  };

  const handleLogout = () => {
    clearStoredCredentials();
    setCredentials(null);
    setBuckets([]);
    setSelectedBucket(null);
    setObjects([]);
    setCommonPrefixes([]);
  };

  // 2. 선택된 버킷의 오브젝트 로드
  const loadObjects = useCallback(async () => {
    if (!selectedBucket) {
      setObjects([]);
      setCommonPrefixes([]);
      return;
    }

    try {
      setIsLoading(true);
      const res = await S3Api.listObjects(selectedBucket, currentPrefix, '/');
      setObjects(res.objects);
      setCommonPrefixes(res.commonPrefixes);

      const currentSum = res.objects.reduce((acc, o) => acc + o.size, 0);
      setTotalObjectsCount(res.objects.length);
      setTotalStorageSize(currentSum);
    } catch (err) {
      console.error(err);
    } finally {
      setIsLoading(false);
    }
  }, [selectedBucket, currentPrefix]);

  useEffect(() => {
    loadObjects();
  }, [loadObjects]);

  const handleSelectBucket = (name: string) => {
    setSelectedBucket(name);
    setCurrentPrefix('');
  };

  const handleCreateBucket = async (name: string) => {
    await S3Api.createBucket(name);
    await loadBuckets();
    setSelectedBucket(name);
    setCurrentPrefix('');
  };

  const handleDeleteBucket = async (name: string) => {
    try {
      await S3Api.deleteBucket(name);
      if (selectedBucket === name) {
        setSelectedBucket(null);
      }
      await loadBuckets();
    } catch (err: any) {
      alert(`Delete bucket failed: ${err.message}`);
    }
  };

  const handleUpload = async (file: File) => {
    if (!selectedBucket) return;
    const key = `${currentPrefix}${file.name}`;
    try {
      await S3Api.uploadObject(selectedBucket, key, file);
      await loadObjects();
    } catch (err: any) {
      alert(`Upload failed: ${err.message}`);
    }
  };

  const handleDeleteObject = async (key: string) => {
    if (!selectedBucket) return;
    try {
      await S3Api.deleteObject(selectedBucket, key);
      await loadObjects();
    } catch (err: any) {
      alert(`Delete failed: ${err.message}`);
    }
  };

  if (!credentials) {
    return <LoginView onLoginSuccess={handleLoginSuccess} />;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
      <Header
        bucketCount={buckets.length}
        totalObjects={totalObjectsCount}
        totalSize={totalStorageSize}
        isRefreshing={isRefreshing}
        currentAccessKey={credentials.accessKey}
        onRefresh={() => {
          loadBuckets();
          loadObjects();
        }}
        onLogout={handleLogout}
      />

      {serverError && (
        <div
          style={{
            padding: '8px 20px',
            backgroundColor: 'var(--status-red-bg)',
            borderBottom: '1px solid rgba(239, 68, 68, 0.3)',
            color: '#f87171',
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            fontSize: '0.75rem',
            fontFamily: 'var(--font-mono)',
          }}
        >
          <AlertCircle size={14} />
          <span>{serverError}</span>
        </div>
      )}

      <div style={{ display: 'flex', flex: 1, overflow: 'hidden' }}>
        <Sidebar
          buckets={buckets}
          selectedBucket={selectedBucket}
          onSelectBucket={handleSelectBucket}
          onOpenCreateModal={() => setIsCreateModalOpen(true)}
          onDeleteBucket={handleDeleteBucket}
        />

        {selectedBucket ? (
          <ObjectExplorer
            bucket={selectedBucket}
            currentPrefix={currentPrefix}
            objects={objects}
            commonPrefixes={commonPrefixes}
            isLoading={isLoading}
            onNavigate={(prefix) => setCurrentPrefix(prefix)}
            onUpload={handleUpload}
            onDelete={handleDeleteObject}
            onPreview={(obj) => setPreviewObject(obj)}
          />
        ) : (
          <div
            style={{
              flex: 1,
              backgroundColor: 'var(--bg-app)',
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--text-muted)',
            }}
          >
            <Database size={40} style={{ opacity: 0.25, marginBottom: '14px' }} />
            <div style={{ fontSize: '1rem', fontWeight: '600', color: 'var(--text-primary)', marginBottom: '4px' }}>
              No bucket selected
            </div>
            <div style={{ fontSize: '0.8125rem', marginBottom: '16px' }}>
              Select a bucket from the left sidebar or create a new one to begin.
            </div>
            <button onClick={() => setIsCreateModalOpen(true)} className="btn btn-primary">
              Create Bucket
            </button>
          </div>
        )}
      </div>

      <CreateBucketModal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        onCreate={handleCreateBucket}
      />

      <PreviewModal
        bucket={selectedBucket || ''}
        object={previewObject}
        onClose={() => setPreviewObject(null)}
      />
    </div>
  );
}

export default App;
