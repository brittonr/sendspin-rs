# Sendspin Server Implementation Plan

**Date**: 2025-12-08

## Executive Summary

Sendspin is a **proprietary WebSocket-based protocol** for synchronized multi-room audio streaming with microsecond-level precision. This is NOT Shoutcast/Icecast - it's a custom protocol designed for hi-res audio (up to 192kHz/24-bit) with sub-10ms synchronization across multiple devices.

## Key Finding

After analyzing both the sendspin-go server and sendspin-rs client codebases:

- **Protocol**: WebSocket with JSON control messages + binary audio chunks
- **Precision**: Microsecond-level clock synchronization using NTP-style 4-timestamp exchange
- **Architecture**: Per-client codec negotiation with independent encoding streams
- **Timing**: 20ms audio chunks sent ahead of playback time with embedded timestamps
- **Roles**: player, controller, metadata, artwork, visualizer (versioned with `@v1`, `@v2`)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Sendspin Server                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ WebSocket   │  │ Client      │  │ Audio Engine            │ │
│  │ Endpoint    │──│ Manager     │──│ (20ms chunk generation) │ │
│  │ /sendspin   │  │             │  │                         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│         │               │                     │                 │
│         ▼               ▼                     ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ Handshake   │  │ Clock Sync  │  │ Per-Client Encoding     │ │
│  │ Handler     │  │ Service     │  │ (PCM/Opus/FLAC)         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│         │               │                     │                 │
│         ▼               ▼                     ▼                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ Audio       │  │ Group       │  │ mDNS Discovery          │ │
│  │ Sources     │  │ Manager     │  │ (optional)              │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Protocol Flow

1. **Connection**:
   - Client connects to WebSocket endpoint (`/sendspin`)
   - Client sends `client/hello` with capabilities
   - Server responds with `server/hello` with active roles

2. **Clock Synchronization** (continuous):
   - Client sends `client/time` with client timestamp (Unix microseconds)
   - Server responds with `server/time` containing:
     - `client_transmitted`: Echo of client timestamp
     - `server_received`: Server loop time when message received
     - `server_transmitted`: Server loop time when response sent
   - Client calculates offset using NTP algorithm

3. **Stream Start**:
   - Server sends `stream/start` with negotiated codec/format
   - Server begins sending binary audio chunks every 20ms

4. **Audio Streaming**:
   - Binary message format: `[type=0x04][timestamp: i64 BE][audio data]`
   - Timestamp = server loop time when audio should play
   - Client converts to local time using clock offset
   - Client schedules audio for playback

5. **State Updates**:
   - Client sends `client/state` with player volume/mute/state
   - Server sends `group/update` with playback state
   - Server sends `server/state` with metadata (if metadata role)

## Project Structure

```
sendspin-rs/
├── src/
│   ├── lib.rs                    # Library exports
│   ├── server/                   # Server implementation (NEW)
│   │   ├── mod.rs
│   │   ├── config.rs             # Server configuration
│   │   ├── server.rs             # Main server loop
│   │   ├── client_handler.rs     # Per-client WebSocket handler
│   │   ├── client_manager.rs     # Connected client registry
│   │   ├── audio_engine.rs       # Audio chunk generation (20ms loop)
│   │   ├── audio_source.rs       # Audio source trait + implementations
│   │   ├── encoder.rs            # Per-client encoding (PCM/Opus/FLAC)
│   │   ├── group.rs              # Group management
│   │   └── clock.rs              # Server clock (monotonic)
│   ├── protocol/                 # (EXISTING) Protocol messages
│   │   ├── mod.rs
│   │   ├── client.rs             # Client implementation
│   │   └── messages.rs           # Message types (EXTEND)
│   ├── audio/                    # (EXISTING) Audio types
│   ├── sync/                     # (EXISTING) Clock sync
│   └── scheduler/                # (EXISTING) Scheduler
├── examples/
│   └── server.rs                 # Example server binary
└── Cargo.toml                    # Dependencies (UPDATE)
```

## Dependencies to Add

```toml
[dependencies]
# WebSocket server
axum = { version = "0.8", features = ["ws"] }

# Audio encoding
opus = "0.3"                      # Opus encoder
flac-bound = "0.2"                # FLAC encoder

# Resampling (for Opus which requires 48kHz)
rubato = "0.15"                   # High-quality resampler

# mDNS discovery (optional)
mdns-sd = "0.11"

# Time handling
chrono = "0.4"

# Tracing/logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

## Implementation Phases

### Phase 1: Core Server Infrastructure ✅ (Design Complete)

- [x] Design architecture
- [ ] Create `server/` module structure
- [ ] Implement `ServerConfig`
- [ ] Implement basic WebSocket endpoint with Axum
- [ ] Implement handshake flow (`client/hello` ↔ `server/hello`)
- [ ] Test: Client can connect and complete handshake

**Files to create**:
- `src/server/mod.rs`
- `src/server/config.rs`
- `src/server/server.rs`
- `src/server/client_handler.rs`

### Phase 2: Client Management

- [ ] Implement `ClientManager` (thread-safe registry)
- [ ] Implement `ConnectedClient` type
- [ ] Add role negotiation logic
- [ ] Add format negotiation logic (PCM/Opus/FLAC)
- [ ] Test: Multiple clients can connect simultaneously

**Files to create**:
- `src/server/client_manager.rs`

### Phase 3: Clock Synchronization

- [ ] Implement server monotonic clock
- [ ] Handle `client/time` messages
- [ ] Generate `server/time` responses
- [ ] Test: Clock sync roundtrip < 1ms

**Files to create**:
- `src/server/clock.rs`

### Phase 4: Audio Engine

- [ ] Create `AudioSource` trait
- [ ] Implement `TestToneSource` (440Hz sine wave)
- [ ] Implement 20ms interval loop
- [ ] Calculate server timestamps
- [ ] Test: Consistent 20ms chunk generation

**Files to create**:
- `src/server/audio_source.rs`
- `src/server/audio_engine.rs`

### Phase 5: Audio Encoding

- [ ] Implement PCM 24-bit encoder (little-endian)
- [ ] Implement Opus encoder (256kbps stereo, 48kHz)
- [ ] Implement FLAC encoder
- [ ] Implement resampler (for Opus)
- [ ] Per-client encoding pipeline
- [ ] Test: Each client receives correct format

**Files to create**:
- `src/server/encoder.rs`

### Phase 6: Binary Message Generation

- [ ] Create binary audio chunk format: `[0x04][timestamp][audio]`
- [ ] Broadcast to all clients
- [ ] Buffer ahead logic (500-1000ms)
- [ ] Test: sendspin-rs client can receive and play

### Phase 7: State Management

- [ ] Extend `messages.rs` with server message types:
  - `GroupUpdate`
  - `ClientState`
  - `StreamClear`
  - `StreamEnd`
  - `ClientGoodbye`
- [ ] Handle `client/state` messages
- [ ] Send `group/update` messages
- [ ] Test: State updates bidirectional

**Files to modify**:
- `src/protocol/messages.rs`

### Phase 8: Group Management

- [ ] Implement `Group` type
- [ ] Implement `GroupManager`
- [ ] Multi-client group logic
- [ ] Volume/mute group operations
- [ ] Test: Multiple clients in same group

**Files to create**:
- `src/server/group.rs`

### Phase 9: Audio Sources (Beyond Test Tone)

- [ ] File source (MP3/FLAC/WAV)
- [ ] HTTP stream source
- [ ] Test: Play real audio files

### Phase 10: mDNS Discovery (Optional)

- [ ] Advertise `_sendspin-server._tcp.local.`
- [ ] TXT record with path
- [ ] Test: Client discovery

### Phase 11: Advanced Features

- [ ] Controller role support (`client/command`)
- [ ] Metadata role support (`server/state`)
- [ ] Artwork role support (binary types 8-11)
- [ ] Visualizer role support (binary type 16)
- [ ] Stream format changes (`stream/request-format`)
- [ ] Seek support (`stream/clear`)

## Key Technical Details

### Binary Message Format

**Audio Chunk** (Player role, type 4):
```
Byte 0:      0x04 (message type)
Bytes 1-8:   timestamp (i64 big-endian, server loop microseconds)
Bytes 9+:    encoded audio frame
```

**Artwork** (Artwork role, types 8-11):
```
Byte 0:      0x08-0x0B (channel 0-3)
Bytes 1-8:   timestamp (i64 big-endian)
Bytes 9+:    encoded image (JPEG/PNG/BMP)
```

### Clock Synchronization Algorithm

```rust
// Client side (for reference)
fn update_clock_sync(t1: i64, t2: i64, t3: i64, t4: i64) {
    // t1 = client_transmitted (Unix µs)
    // t2 = server_received (server loop µs)
    // t3 = server_transmitted (server loop µs)
    // t4 = client_received (Unix µs)

    let rtt = (t4 - t1) - (t3 - t2);

    // Discard high-RTT samples (>100ms indicates network issues)
    if rtt > 100_000 {
        return;
    }

    // Calculate server loop start time in Unix epoch
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    let server_loop_start_unix = now_unix - t2;

    // Store offset for timestamp conversion
}
```

### Audio Encoding

**PCM 24-bit** (3 bytes per sample, little-endian):
```rust
fn encode_pcm_24bit(samples: &[i32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 3);
    for &sample in samples {
        out.push((sample & 0xFF) as u8);
        out.push(((sample >> 8) & 0xFF) as u8);
        out.push(((sample >> 16) & 0xFF) as u8);
    }
    out
}
```

**Opus** (256kbps stereo, 48kHz required):
- Resample to 48kHz if source is different
- Use `opus` crate with `Audio` application mode
- 20ms frames (960 samples at 48kHz)

**FLAC**:
- Use `flac-bound` crate
- Send codec header in `stream/start` message
- Lossless compression

### Role Negotiation

Client sends in `client/hello`:
```json
{
  "supported_roles": ["player@v2", "player@v1", "controller@v1"]
}
```

Server activates first match per role family:
- If server supports `player@v2`: activate `player@v2`
- If server only supports `player@v1`: activate `player@v1`
- Always activate `controller@v1` if client requests it

Server responds in `server/hello`:
```json
{
  "active_roles": ["player@v1", "controller@v1"]
}
```

### Format Negotiation

Client sends in `client/hello.player@v1_support`:
```json
{
  "supported_formats": [
    {"codec": "opus", "channels": 2, "sample_rate": 48000, "bit_depth": 16},
    {"codec": "pcm", "channels": 2, "sample_rate": 48000, "bit_depth": 24},
    {"codec": "pcm", "channels": 2, "sample_rate": 96000, "bit_depth": 24}
  ]
}
```

Server picks first supported format and sends in `stream/start`:
```json
{
  "player": {
    "codec": "opus",
    "sample_rate": 48000,
    "channels": 2,
    "bit_depth": 16
  }
}
```

## Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Audio latency | < 10ms jitter | Synchronized playback requirement |
| CPU usage | < 5% per client | Efficient per-client encoding |
| Memory | < 50MB + 5MB/client | Reasonable overhead |
| Max clients | 50+ concurrent | Multi-room scenarios |
| Clock sync precision | < 1ms RTT | Microsecond-level sync |

## Testing Strategy

### Unit Tests
- Message serialization/deserialization
- Clock sync calculations
- Audio encoding correctness
- Binary message format validation

### Integration Tests
- Full handshake flow
- Clock synchronization roundtrip
- Audio chunk delivery
- Multi-client scenarios

### End-to-End Tests
- sendspin-rs client ↔ sendspin-rs server
- Audio playback verification
- Synchronization accuracy measurement

## Reference Implementation Analysis

### From sendspin-go Server

**Key Files Analyzed**:
- `/internal/server/server.go` (613 lines): Main server loop, WebSocket handling
- `/internal/server/audio_engine.go` (358 lines): 20ms chunk generation
- `/internal/server/audio_source.go` (581 lines): MP3/FLAC/test tone sources
- `/internal/server/opus_encoder.go` (65 lines): Opus encoding
- `/pkg/sync/clock.go`: Clock synchronization
- `/pkg/sendspin/scheduler.go` (202 lines): Playback scheduling

**Go Implementation Details**:
- Uses `gorilla/websocket` for WebSocket server
- Per-client goroutine for message handling
- 20ms ticker with `time.NewTicker(20 * time.Millisecond)`
- Opus encoding at 256kbps using `gopus`
- Linear interpolation resampler for 192kHz ↔ 48kHz
- mDNS using `github.com/hashicorp/mdns`

### From sendspin-rs Client

**Key Files Used**:
- `/src/protocol/client.rs` (269 lines): WebSocket client
- `/src/protocol/messages.rs` (188 lines): Message types
- `/src/audio/types.rs` (106 lines): 24-bit Sample type
- `/src/sync/clock.rs` (134 lines): NTP-style clock sync
- `/src/scheduler/audio_scheduler.rs` (72 lines): Lock-free scheduler

**Client Expectations**:
- WebSocket endpoint at `/sendspin`
- JSON text messages for control
- Binary messages for audio (type byte + timestamp + data)
- 20ms chunks with server timestamps
- Clock sync every ~5 seconds

## Open Questions

1. **Audio Source Priority**: Start with test tone or file playback?
   - **Decision**: Test tone first (simpler), file playback in Phase 9

2. **mDNS**: Required or optional?
   - **Decision**: Optional (Phase 10), allow manual connection

3. **Multi-server Support**: Implement `connection_reason` logic?
   - **Decision**: Phase 11 (advanced)

4. **Codec Priority**: PCM-only first or all three codecs?
   - **Decision**: PCM first (Phase 4-6), Opus/FLAC in Phase 5

5. **Threading Model**: How many threads/tasks?
   - **Decision**:
     - 1x Axum runtime (WebSocket connections)
     - 1x Audio engine task (20ms loop)
     - Nx Client handler tasks (one per client)

## Success Criteria

- [ ] sendspin-rs client can connect to sendspin-rs server
- [ ] Client receives synchronized audio chunks
- [ ] Clock sync achieves <1ms RTT
- [ ] Audio plays with <10ms jitter
- [ ] Multiple clients can connect and play in sync
- [ ] All message types implemented per spec.md
- [ ] Server compiles and runs on NixOS

## Next Steps

1. Create `src/server/` directory structure
2. Implement Phase 1: Core Server Infrastructure
3. Test handshake with existing sendspin-rs client
4. Iterate through phases 2-11

## Notes

- The Go server is ~1800 LOC (core), Rust equivalent should be similar
- Existing sendspin-rs client code is reusable for message types
- Focus on correctness first, then performance optimization
- Use `tracing` for structured logging from the start
- All timestamps in microseconds (i64)
- Server clock is monotonic (from server start), not Unix epoch
