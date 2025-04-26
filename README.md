# tts-gateway

## Introduction

This is a simple tts gateway with optional caching written in Rust.

## Getting started

```dockerfile
FROM tombailey256/tts-gateway:0.0.0

ENV PORT=8080
```

See Docker Hub for the latest release tags:
[https://hub.docker.com/r/tombailey256/tts-gateway](https://hub.docker.com/r/tombailey256/tts-gateway)

## Configure services

### GCP

Uses a [GCP TTS](https://cloud.google.com/text-to-speech/docs) voice for speech.

```shell
export GOOGLE_API_KEY="..."
```

### OpenAI

Uses an [OpenAI model](https://platform.openai.com/docs/models#tts) for speech.

```shell
export OPENAI_API_KEY="..."
```

## Configure cache

### R2

Uses an [R2](https://www.cloudflare.com/en-gb/developer-platform/products/r2/) bucket for caching responses.

```shell
export R2_BUCKET_NAME="..."
export R2_BUCKET_PREFIX="tts/cache"
export R2_ACCESS_KEY_ID="..."
export R2_ACCESS_KEY_SECRET="..."
export R2_ACCOUNT_ID="..."
```

If you have an EU based bucket, set:
```shell
export R2_ENDPOINT="eu.r2.cloudflarestorage.com"
```

### S3

Uses an [S3](https://aws.amazon.com/s3/) bucket for caching responses.

```shell
export S3_BUCKET_NAME="..."
export S3_BUCKET_PREFIX="tts/cache"
export S3_REGION="us-west-2"
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_SESSION_TOKEN="..."
```

## Rest API

### Generate speech

#### GCP

```shell
curl -X POST -H "Content-Type: application/json" http://localhost:8080/v1/speech/gcp -d '{ "text": "hello world", "voice": { "name": "en-US-Standard-A", "languageCode": "en-US" }, "audioConfig": { "audioEncoding": "OGG_OPUS" } }'
# 200 OK
# [audio byte data...]
```

Google provide a list of voices and language codes for their TTS service:
https://cloud.google.com/text-to-speech/docs/list-voices-and-types

The following audio encodings are supported:
- `LINEAR16`
- `MP3`
- `OGG_OPUS`
- `MULAW`
- `ALAW`
- `PCM`

#### OpenAI

```shell
curl -X POST -H "Content-Type: application/json" http://localhost:8080/v1/speech/openai -d '{ "text": "hello world", "model": "gpt-4o-mini-tts", "voice": "echo", "responseFormat": "mp3" }'
# 200 OK
# [audio byte data...]
```

The following models are supported:
- `gpt-4o-mini-tts`
- `tts-1`
- `tts-1-hd`

The following voices are supported:
- `alloy`
- `ash`
- `ballad`
- `coral`
- `echo`
- `fable`
- `onyx`
- `nova`
- `sage`
- `shimmer`
- `verse`

The following response formats are supported:
- `mp3`
- `opus`
- `aac`
- `flac`
- `wav`
- `pcm`

## Health check

A built-in health check endpoint (`/health`) confirms that the server is running. It does NOT verify the health of any cache or TTS services.
