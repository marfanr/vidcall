## Models

Data structures and database connection management.

### Files

| File | Description |
|------|-------------|
| `mod.rs` | Module exports and `StandardModel` trait |
| `room.rs` | Room, RoomId, Peer models |
| `conn.rs` | PostgreSQL connection pool initialization |

### RoomId

A unique room identifier using nanoid (12 characters, alphanumeric + `_` + `-`).

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RoomId(Arc<str>);
```

### Room

Represents a video call room with peers.

```rust
pub struct Room {
    pub id: RoomId,
    pub name: RwLock<Arc<str>>,
    pub peers: ArcSwap<Vec<Peer>>
}
```

### Peer

Represents a participant in a room.

```rust
pub struct Peer {
    pub id: Uuid,
    pub ws_tx: mpsc::Sender<Message>,
    pub pc: Box<dyn PeerConnection>,
}
```

### Database Connection

Uses `deadpool_postgres` with TLS for PostgreSQL connection pooling. Configuration via environment variables:
- `POSTGRES_PASSWORD` - Database password
- `POSTGRES_PORT` - Database port (default: 5432)
- `PG_CA_CERT` - CA certificate path (default: `certs/ca.crt`)

### Dependencies

- `deadpool_postgres` - Async PostgreSQL pool
- `tokio_postgres_rustls` - Rustls-based TLS for PostgreSQL
- `webrtc` - WebRTC peer connection
- `arc_swap` - Atomic swaps for Arc pointers
- `nanoid` - URL-safe unique ID generation
