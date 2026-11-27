# Core Events Stream

observed on 2026-11-27 with `hc 0.1.22` against an isolated scratch core. piped `hc events` prints one JSON object per line. its first line is a snapshot with the current terminals.

```json
{"channel":null,"seq":0,"terminals":[{"address":"t1","state":"live"}]}
```

subsequent lines carry events in the JSON-RPC envelope. these examples omit fields the tile does not use.

```json
{"jsonrpc":"2.0","method":"event","params":{"data":{"address":"t1","state":"live"},"seq":1,"terminal":"t1","type":"created"}}
{"jsonrpc":"2.0","method":"event","params":{"data":{"argv":["sleep","10"],"name":"sleep","pid":96109},"seq":2,"terminal":"t1","type":"foreground"}}
{"jsonrpc":"2.0","method":"event","params":{"data":{"exit_code":null,"finished_ms":1795772868251,"reason":"killed","signal":1},"seq":3,"terminal":"t1","type":"finished"}}
```

seed the live set with snapshot addresses whose state is `live`. add live `created` addresses and remove the address on `finished`. the tile also handles `removed` if it appears and ignores other event types. the snapshot supplies the initial count without a separate listing.

`hc server stop` makes the events process exit and closes the stream.
