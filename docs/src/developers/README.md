# For Developers

TODO: jot down design notions and useful information for folks aiming to help contribute to or customize this software.

`fossilizer` has not yet been published as a crate, but you can see the module docs here:

- [Crate fossilizer](../doc/fossilizer/index.html)

## Odds & Ends

- For some details on how SQLite is used here as an ad-hoc document database, check out this blog post on [Using SQLite as a document database for Mastodon exports](https://blog.lmorchard.com/2023/05/12/toots-in-sqlite/)

- When ingesting data, care is taken to attempt to store JSON as close to the original source as possible from APIs and input files. That way, parsing and data models can incrementally upgraded over time without having lost any information from imported sources.
