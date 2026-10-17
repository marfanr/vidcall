## API v1

API version 1 submodules.

### Modules

| Module | Description |
|--------|-------------|
| `auth` | Authentication (signin, refresh token) |
| `user` | User management (registration) |
| `room` | Room management and WebSocket |

### Routes

All routes are mounted under `/v1/`:
- `/v1/auth/*` - Authentication endpoints
- `/v1/user/*` - User endpoints
- `/v1/room/*` - Room endpoints (including WebSocket)

### Notes

This is a re-export of modules from `src/api/`. See [`src/api/readme.md`](../readme.md) for full API documentation.
