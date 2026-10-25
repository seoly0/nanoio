# nanoio

A lightweight, high-performance S3-compatible object storage server written in Rust.

`nanoio` is designed specifically for **local development, CI/CD pipelines, and edge testing environments** where running a heavyweight distributed storage cluster is unnecessary.

[![Docker Image](https://img.shields.io/badge/Docker%20Hub-seoly0%2Fnanoio-blue?logo=docker)](https://hub.docker.com/r/seoly0/nanoio)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-linux%2Famd64%20%7C%20linux%2Farm64-lightgrey)](#)

> [!WARNING]
> **Not Intended for Production**  
> `nanoio` is built strictly for local development and testing. It stores objects directly on a local filesystem without distributed replication, erasure coding, or automated failover. **Using `nanoio` as a production data store is strongly discouraged.** For production environments, use services like AWS S3, Cloudflare R2, or enterprise distributed systems (e.g., Ceph).

---

## Features

- **S3 REST API Compatibility (Port 9000)**
  - Bucket operations: Create, List, Delete, and Head
  - Object operations: Put, Get, Head, Delete, and Object Copy (`x-amz-copy-source`)
  - Multipart Upload: `InitiateMultipartUpload`, `UploadPart`, `CompleteMultipartUpload`, and `AbortMultipartUpload`
  - Batch Delete: `POST /{bucket}?delete`
  - Byte-Range Requests: `Range` header support for partial downloads and streaming (`206 Partial Content`)
  - Virtual Folder Hierarchy: `prefix` and `delimiter` handling (`CommonPrefixes`)

- **Authentication & IAM**
  - AWS Signature Version 4 (SigV4) validation using HMAC-SHA256
  - Presigned URL support with expiration (`X-Amz-Expires`) validation
  - Multi-user IAM support with granular policies (`Admin`, `ReadWrite`, `ReadOnly`)
  - Standard S3 XML error responses (`SignatureDoesNotMatch`, `InvalidAccessKeyId`, `AccessDenied`)
  - Configurable authentication modes (`strict` or `permissive`)

- **Ultra-lightweight Architecture**
  - Minimal image size (~33 MB)
  - Low memory footprint (~20 MB RSS at idle)
  - Instant startup with zero external runtime dependencies

---

## Quick Start

The pre-built multi-arch Docker image (`linux/amd64` and `linux/arm64`) is available on [Docker Hub](https://hub.docker.com/r/seoly0/nanoio).

```bash
docker pull seoly0/nanoio:latest
```

### 1. Using Docker CLI

```bash
docker run -d \
  --name nanoio \
  -p 9000:9000 \
  -e NANOIO_ACCESS_KEY_ID=<YOUR_ACCESS_KEY> \
  -e NANOIO_SECRET_ACCESS_KEY=<YOUR_SECRET_KEY> \
  -e NANOIO_DEFAULT_BUCKET=my-bucket \
  -v $(pwd)/data:/data \
  seoly0/nanoio:latest
```

### 2. Using Docker Compose

```yaml
services:
  s3:
    image: seoly0/nanoio:latest
    container_name: nanoio
    ports:
      - "9000:9000"     # S3 API
    environment:
      - NANOIO_AUTH_MODE=strict
      - NANOIO_ACCESS_KEY_ID=<YOUR_ACCESS_KEY>
      - NANOIO_SECRET_ACCESS_KEY=<YOUR_SECRET_KEY>
      # - NANOIO_DEFAULT_BUCKET=my-bucket # Optional: Auto-create bucket on startup
      - NANOIO_DATA_DIR=/data
    volumes:
      - ./data:/data
    restart: unless-stopped
```

Run:

```bash
docker compose up -d
```

- **S3 API Endpoint**: `http://localhost:9000`

---

## Configuration

The server can be configured via environment variables or command-line arguments:

| Environment Variable | CLI Argument | Default | Description |
| :--- | :--- | :--- | :--- |
| `NANOIO_HOST` | `-H`, `--host` | `0.0.0.0` | Bind IP address |
| `NANOIO_PORT` | `-p`, `--port` | `9000` | S3 API listening port |
| `NANOIO_DATA_DIR` | `-d`, `--data-dir` | `./data` | Local directory for storing objects and metadata |
| `NANOIO_AUTH_MODE` | `--auth-mode` | `strict` | `strict` (validates SigV4) or `permissive` (pass-through) |
| `NANOIO_ACCESS_KEY_ID` | `--access-key-id` | `nanoioadmin` | Root access key ID |
| `NANOIO_SECRET_ACCESS_KEY` | `--secret-access-key` | `nanoioadmin` | Root secret access key |
| `NANOIO_DEFAULT_BUCKET` | `--default-bucket` | `None` | Comma-separated buckets to auto-create on startup |
| `NANOIO_IAM_USERS` | `--iam-users` | `None` | Comma-separated IAM users: `key:secret:policy,...` |
| `RUST_LOG` | `--log-level` | `info` | Log verbosity (`trace`, `debug`, `info`, `warn`, `error`) |

### IAM Users & Policies

You can define additional IAM users via `NANOIO_IAM_USERS`. Each user entry follows the format `<access_key>:<secret_key>[:<policy>]`:

```bash
NANOIO_IAM_USERS="reader:reader123:readonly,dev:dev123:readwrite"
```

Available policies:
- `Admin`: Full access to all bucket and object operations.
- `ReadWrite`: Object upload, download, and deletion allowed. Bucket deletion restricted.
- `ReadOnly`: Only `GET`, `HEAD`, and list requests allowed. Write operations return `403 AccessDenied`.

---

## Client Usage Examples

### AWS CLI

```bash
export AWS_ACCESS_KEY_ID=<YOUR_ACCESS_KEY>
export AWS_SECRET_ACCESS_KEY=<YOUR_SECRET_KEY>
export AWS_REGION=us-east-1

# Create a bucket
aws --endpoint-url http://localhost:9000 s3 mb s3://my-bucket

# Upload a file
aws --endpoint-url http://localhost:9000 s3 cp ./document.pdf s3://my-bucket/

# List objects
aws --endpoint-url http://localhost:9000 s3 ls s3://my-bucket/
```

### Python (boto3)

```python
import boto3

s3 = boto3.client(
    's3',
    endpoint_url='http://localhost:9000',
    aws_access_key_id='<YOUR_ACCESS_KEY>',
    aws_secret_access_key='<YOUR_SECRET_KEY>',
    region_name='us-east-1'
)

# Create bucket and upload
s3.create_bucket(Bucket='test-bucket')
s3.put_object(Bucket='test-bucket', Key='hello.txt', Body=b'Hello from nanoio')

# Generate a presigned download URL
url = s3.generate_presigned_url(
    'get_object',
    Params={'Bucket': 'test-bucket', 'Key': 'hello.txt'},
    ExpiresIn=3600
)
print("Presigned URL:", url)
```

### Node.js (@aws-sdk/client-s3)

```typescript
import { S3Client, PutObjectCommand, ListObjectsV2Command } from '@aws-sdk/client-s3';

const client = new S3Client({
  endpoint: 'http://localhost:9000',
  region: 'us-east-1',
  credentials: {
    accessKeyId: '<YOUR_ACCESS_KEY>',
    secretAccessKey: '<YOUR_SECRET_KEY>',
  },
  forcePathStyle: true,
});

await client.send(new PutObjectCommand({
  Bucket: 'test-bucket',
  Key: 'sample.json',
  Body: JSON.stringify({ message: 'ok' }),
  ContentType: 'application/json',
}));
```


---

## License

This project is licensed under the [MIT License](LICENSE).
