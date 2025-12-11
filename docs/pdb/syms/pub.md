## `S_PUB32` (0x110e) - Public Symbol

```c
struct PubSym {
    uint32_t flags;
    uint32_t offset;
    uint16_t segment;
    strz name;
};
```

`S_PUB32` should only appear in the Global Symbol Stream.
