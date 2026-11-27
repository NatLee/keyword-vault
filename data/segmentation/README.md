# Traditional Chinese preview dictionaries

`dict.txt.big` is copied verbatim from the MIT-licensed [Jieba dictionary](https://github.com/fxsjy/jieba/blob/67fa2e36e72f69d9134b8a1037b83fbb070b9775/extra_dict/dict.txt.big), revision `67fa2e36e72f69d9134b8a1037b83fbb070b9775`.

- Size: 8,583,143 bytes.
- SHA-256: `b16011275c42955ccd81fc1adecc93a59dbb7926af69d93fc95d4943d40f6aad`.
- Original license is in `LICENSE`.
- `taiwan.txt` is this project's explicit domain supplement (MIT), profile `tw-v1`.

The API identifies the effective base + supplement + optional custom dictionary by a SHA-256 of their original bytes separated by NUL bytes. It uses jieba-rs 0.11.0 with HMM disabled. This is a local dictionary preview, not an Elasticsearch analyzer. Keep words in the same NFKC form used by the expression compiler.

Set `KV_SEGMENTATION_DICT` to a deployment-owned UTF-8 file for custom entries, one `word frequency [tag]` per line. Limits: 1 MiB, 10,000 nonblank entries, 1–100 Unicode scalars per word, positive frequency at most 1,000,000,000. Duplicate words within the custom file and malformed rows fail startup; words may intentionally override the bundled dictionary. Restart after a dictionary change. The service does not accept dictionary paths from clients.
