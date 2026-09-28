"mdt_core": fix
---

# Feed formatter stdin from a dedicated thread

Formatter commands received their full input through the pipe before stdout was drained. A formatter that fills its stdout pipe while mdt is still writing a large file deadlocked both processes. stdin is now written from a spawned thread while the main thread reads output, and a formatter that exits early is reported through its exit status instead of a broken-pipe I/O error.
