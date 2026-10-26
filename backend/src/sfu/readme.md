## SFU (Selective Forwarding Unit)

> **Status:** Placeholder - Not yet implemented

This module is intended for SFU (Selective Forwarding Unit) implementation to handle multi-participant video calls more efficiently than mesh topology.

### Planned Features

- [x] Media routing/switching
- [ ] Recording support
- [ ] Transcoding
- [ ] Scalability for many participants

### Note

Currently, the application uses mesh topology where each peer connects directly to every other peer. This is suitable for small groups (2-4 participants) but does not scale well for larger rooms.

The SFU module will be implemented to replace the current peer-to-peer model when needed.
