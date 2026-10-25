export interface S3Bucket {
  name: string;
  creationDate: string;
}

export interface S3Object {
  key: string;
  size: number;
  etag: string;
  lastModified: string;
  storageClass: string;
}

export interface ListObjectsResponse {
  name: string;
  prefix: string;
  objects: S3Object[];
  commonPrefixes: string[];
}

export const S3_ENDPOINT = 'http://localhost:9000';

export interface Credentials {
  accessKey: string;
  secretKey: string;
}

const STORAGE_KEY_AK = 'nanoio_session_access_key';
const STORAGE_KEY_SK = 'nanoio_session_secret_key';

export function getStoredCredentials(): Credentials | null {
  const ak = localStorage.getItem(STORAGE_KEY_AK);
  const sk = localStorage.getItem(STORAGE_KEY_SK);
  if (ak && sk) {
    return { accessKey: ak, secretKey: sk };
  }
  return null;
}

export function setStoredCredentials(creds: Credentials) {
  localStorage.setItem(STORAGE_KEY_AK, creds.accessKey);
  localStorage.setItem(STORAGE_KEY_SK, creds.secretKey);
}

export function clearStoredCredentials() {
  localStorage.removeItem(STORAGE_KEY_AK);
  localStorage.removeItem(STORAGE_KEY_SK);
}

// Web Crypto API 기반 AWS SigV4 서명 생성 헬퍼
async function hmacSha256(key: ArrayBuffer | Uint8Array, data: string): Promise<ArrayBuffer> {
  const keyData = (key instanceof Uint8Array ? key : new Uint8Array(key)) as unknown as BufferSource;
  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    keyData,
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  return await crypto.subtle.sign('HMAC', cryptoKey, new TextEncoder().encode(data));
}

async function sha256Hex(data: string | Uint8Array): Promise<string> {
  const buffer = (typeof data === 'string' ? new TextEncoder().encode(data) : data) as unknown as BufferSource;
  const hash = await crypto.subtle.digest('SHA-256', buffer);
  return Array.from(new Uint8Array(hash))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

async function bufferToHex(buffer: ArrayBuffer): Promise<string> {
  return Array.from(new Uint8Array(buffer))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

async function getSigV4Headers(
  method: string,
  path: string,
  queryString: string = '',
  customCreds?: Credentials
): Promise<Record<string, string>> {
  const creds = customCreds || getStoredCredentials() || {
    accessKey: 'nanoioadmin',
    secretKey: 'nanoioadmin',
  };

  const now = new Date();
  const amzDate = now.toISOString().replace(/[:-]|\.\d{3}/g, '');
  const dateStamp = amzDate.slice(0, 8);
  const region = 'us-east-1';
  const service = 's3';

  // 1. Signing Key 생성
  const kSecret = new TextEncoder().encode(`AWS4${creds.secretKey}`);
  const kDate = await hmacSha256(kSecret, dateStamp);
  const kRegion = await hmacSha256(kDate, region);
  const kService = await hmacSha256(kRegion, service);
  const kSigning = await hmacSha256(kService, 'aws4_request');

  // 2. Canonical Request
  const canonicalHeaders = `x-amz-date:${amzDate}\n`;
  const signedHeaders = 'x-amz-date';
  const payloadHash = 'UNSIGNED-PAYLOAD';

  const canonicalRequest = `${method}\n${path}\n${queryString}\n${canonicalHeaders}\n${signedHeaders}\n${payloadHash}`;
  const hashedCanonical = await sha256Hex(canonicalRequest);

  // 3. String to Sign
  const scope = `${dateStamp}/${region}/${service}/aws4_request`;
  const stringToSign = `AWS4-HMAC-SHA256\n${amzDate}\n${scope}\n${hashedCanonical}`;

  // 4. Signature
  const signatureBuffer = await hmacSha256(kSigning, stringToSign);
  const signature = await bufferToHex(signatureBuffer);

  const authHeader = `AWS4-HMAC-SHA256 Credential=${creds.accessKey}/${scope}, SignedHeaders=${signedHeaders}, Signature=${signature}`;

  return {
    'x-amz-date': amzDate,
    'Authorization': authHeader,
  };
}

export class S3Api {
  static async validateCredentials(creds: Credentials): Promise<{ valid: boolean; error?: string }> {
    try {
      const sigHeaders = await getSigV4Headers('GET', '/', '', creds);
      const res = await fetch(`${S3_ENDPOINT}/`, {
        method: 'GET',
        headers: sigHeaders,
      });

      if (res.ok) {
        return { valid: true };
      }

      const text = await res.text();
      const parser = new DOMParser();
      const xmlDoc = parser.parseFromString(text, 'application/xml');
      const code = xmlDoc.querySelector('Code')?.textContent || res.statusText;
      const message = xmlDoc.querySelector('Message')?.textContent || 'Authentication failed';

      if (code === 'InvalidAccessKeyId') {
        return { valid: false, error: 'Access Key ID does not exist' };
      }
      if (code === 'SignatureDoesNotMatch') {
        return { valid: false, error: 'Secret Access Key is incorrect (Signature mismatch)' };
      }
      return { valid: false, error: `${code}: ${message}` };
    } catch (err: any) {
      return { valid: false, error: `Failed to connect to storage: ${err.message}` };
    }
  }

  private static async authedFetch(
    method: string,
    path: string,
    queryString: string = '',
    customHeaders: Record<string, string> = {},
    body?: BodyInit
  ): Promise<Response> {
    const url = queryString ? `${S3_ENDPOINT}${path}?${queryString}` : `${S3_ENDPOINT}${path}`;
    const sigHeaders = await getSigV4Headers(method, path, queryString);

    return fetch(url, {
      method,
      headers: {
        ...customHeaders,
        ...sigHeaders,
      },
      body,
    });
  }

  static async listBuckets(): Promise<S3Bucket[]> {
    const res = await this.authedFetch('GET', '/');
    if (!res.ok) throw new Error(`Failed to list buckets: ${res.statusText}`);
    const xmlText = await res.text();
    const parser = new DOMParser();
    const xmlDoc = parser.parseFromString(xmlText, 'application/xml');

    const bucketNodes = xmlDoc.querySelectorAll('Bucket');
    const buckets: S3Bucket[] = [];
    bucketNodes.forEach((node) => {
      const name = node.querySelector('Name')?.textContent || '';
      const creationDate = node.querySelector('CreationDate')?.textContent || '';
      if (name) buckets.push({ name, creationDate });
    });
    return buckets;
  }

  static async createBucket(bucketName: string): Promise<void> {
    const res = await this.authedFetch('PUT', `/${bucketName}`);
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Failed to create bucket: ${res.statusText} (${errText})`);
    }
  }

  static async deleteBucket(bucketName: string): Promise<void> {
    const res = await this.authedFetch('DELETE', `/${bucketName}`);
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Failed to delete bucket: ${res.statusText} (${errText})`);
    }
  }

  static async listObjects(
    bucketName: string,
    prefix: string = '',
    delimiter: string = '/'
  ): Promise<ListObjectsResponse> {
    const query = new URLSearchParams({
      'list-type': '2',
      prefix,
      delimiter,
    });

    const res = await this.authedFetch('GET', `/${bucketName}`, query.toString());
    if (!res.ok) throw new Error(`Failed to list objects: ${res.statusText}`);
    const xmlText = await res.text();
    const parser = new DOMParser();
    const xmlDoc = parser.parseFromString(xmlText, 'application/xml');

    const objects: S3Object[] = [];
    xmlDoc.querySelectorAll('Contents').forEach((node) => {
      const key = node.querySelector('Key')?.textContent || '';
      const size = parseInt(node.querySelector('Size')?.textContent || '0', 10);
      const etag = node.querySelector('ETag')?.textContent || '';
      const lastModified = node.querySelector('LastModified')?.textContent || '';
      const storageClass = node.querySelector('StorageClass')?.textContent || 'STANDARD';
      if (key) objects.push({ key, size, etag, lastModified, storageClass });
    });

    const commonPrefixes: string[] = [];
    xmlDoc.querySelectorAll('CommonPrefixes > Prefix').forEach((node) => {
      const p = node.textContent || '';
      if (p) commonPrefixes.push(p);
    });

    return {
      name: bucketName,
      prefix,
      objects,
      commonPrefixes,
    };
  }

  static async uploadObject(
    bucketName: string,
    key: string,
    file: File | Blob,
    contentType?: string
  ): Promise<void> {
    const headers: Record<string, string> = {};
    if (contentType || (file instanceof File && file.type)) {
      headers['Content-Type'] = contentType || (file as File).type;
    }

    const res = await this.authedFetch('PUT', `/${bucketName}/${key}`, '', headers, file);
    if (!res.ok) {
      throw new Error(`Upload failed: ${res.statusText}`);
    }
  }

  static async deleteObject(bucketName: string, key: string): Promise<void> {
    const res = await this.authedFetch('DELETE', `/${bucketName}/${key}`);
    if (!res.ok) throw new Error(`Delete failed: ${res.statusText}`);
  }

  static async getPresignedDownloadUrl(bucketName: string, key: string, expiresIn: number = 3600): Promise<string> {
    const creds = getStoredCredentials() || { accessKey: 'nanoioadmin', secretKey: 'nanoioadmin' };
    const now = new Date();
    const amzDate = now.toISOString().replace(/[:-]|\.\d{3}/g, '');
    const dateStamp = amzDate.slice(0, 8);
    const region = 'us-east-1';
    const service = 's3';

    const kSecret = new TextEncoder().encode(`AWS4${creds.secretKey}`);
    const kDate = await hmacSha256(kSecret, dateStamp);
    const kRegion = await hmacSha256(kDate, region);
    const kService = await hmacSha256(kRegion, service);
    const kSigning = await hmacSha256(kService, 'aws4_request');

    const credential = `${creds.accessKey}/${dateStamp}/${region}/${service}/aws4_request`;
    const queryParams: Record<string, string> = {
      'X-Amz-Algorithm': 'AWS4-HMAC-SHA256',
      'X-Amz-Credential': credential,
      'X-Amz-Date': amzDate,
      'X-Amz-Expires': expiresIn.toString(),
      'X-Amz-SignedHeaders': 'host',
    };

    const sortedQuery = Object.keys(queryParams)
      .sort()
      .map((k) => `${encodeURIComponent(k)}=${encodeURIComponent(queryParams[k])}`)
      .join('&');

    const path = `/${bucketName}/${key}`;
    const hostHeader = 'localhost:9000';
    const canonicalReq = `GET\n${path}\n${sortedQuery}\nhost:${hostHeader}\n\nhost\nUNSIGNED-PAYLOAD`;
    const hashedCanonical = await sha256Hex(canonicalReq);

    const stringToSign = `AWS4-HMAC-SHA256\n${amzDate}\n${dateStamp}/${region}/${service}/aws4_request\n${hashedCanonical}`;
    const sigBuffer = await hmacSha256(kSigning, stringToSign);
    const signature = await bufferToHex(sigBuffer);

    return `${S3_ENDPOINT}${path}?${sortedQuery}&X-Amz-Signature=${signature}`;
  }

  static getObjectUrl(bucketName: string, key: string): string {
    return `${S3_ENDPOINT}/${bucketName}/${key}`;
  }
}
