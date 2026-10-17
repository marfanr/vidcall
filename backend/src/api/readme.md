## API

This folder handles the API Server and WebSocket for the video call application, including routing and handlers/controllers.

### Structure

| File | Description |
|------|-------------|
| `mod.rs` | Shared utilities: `ApiError`, `ValidateJson`, `FastJson`, error helpers |
| `server.rs` | Server setup with Axum router, CORS, state management |
| `auth.rs` | Authentication endpoints |
| `user.rs` | User management endpoints |
| `room.rs` | Room management and WebSocket handling |

### Server State (`ServerState`)

```rust
pub struct ServerState {
    pub rooms: Arc<DashMap<RoomId, Arc<Room>>>,           // Active rooms
    pub connections: Arc<DashMap<uuid::Uuid, mpsc::Sender<Message>>>, // WS connections
    pub pcs: Arc<RwLock<HashMap<uuid::Uuid, Arc<dyn PeerConnection>>>>, // WebRTC peer connections
    pub pool: Pool,                                        // PostgreSQL connection pool
}
```

### Endpoints

#### Auth (`/v1/auth`)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/signin` | Sign in with email & password, returns JWT token |
| `POST` | `/refresh` | Refresh JWT using refresh token from cookie |

#### User (`/v1/user`)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/` | Create new user (username, email, password) |

#### Room (`/v1/room`)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/` | Create new room |
| `GET` | `/ws` | WebSocket for room communication |

### WebSocket Messages

Client -> Server:

```json
{ "type": "join", "room": "room_id" }
{ "type": "offer", "sdp": "..." }
{ "type": "answer", "_sdp": "..." }
{ "type": "candidate", "candidate": "...", "mid": "...", "line_index": 0, "username_fragment": "..." }
{ "type": "leave" }
```

Server -> Client:

```json
{ "type": "answer", "sdp": "..." }
{ "type": "ice", "candidate": "...", "sdpMid": "...", "sdpMLineIndex": 0, "usernameFragment": "..." }
```

### Error Handling

`ApiError` variants:
- `NotFound` - 500 internal error
- `Validation(ValidationErrors)` - 422 with field validation errors
- `Rejection(StatusCode, String)` - Custom error with status code

### Dependencies

- **axum** - Web framework
- **tokio_postgres / deadpool_postgres** - PostgreSQL async pool
- **webrtc** - WebRTC peer connection
- **argon2** - Password hashing
- **sonic_rs** - Fast JSON serialization
- **tower_http** - HTTP middleware (CORS, tracing)
