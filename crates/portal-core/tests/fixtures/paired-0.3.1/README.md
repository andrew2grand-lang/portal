Two data folders paired by SeamDrop 0.3.1 (protocol 2, numeric-comparison
pairing), saved before Mission 04 changed anything. `tests/old_pairing.rs`
starts today's core from copies of them to prove existing pairings keep
working. `key.der` is the raw private key (throwaway, test-only); the test
seals it the way each OS stores it (`key.bin`).
