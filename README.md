# fossilizer

This is an attempt to build a static site generator for Mastodon export tarballs.

## TODO

- [ ] move all of these to-dos to issues
- [x] fetch outbox url for incremental imports
- [ ] use mastodon API for authorized fetch
- [ ] include actor info in site header?
- [ ] build documented types for template contexts
- [ ] special index just for activities with media attachments?
- [ ] navigation between media attachments in dialog, prev & next
- [ ] improve error handling in general
- [ ] GitHub pages publish action?
- [ ] CI automation on github for tests
- [ ] automated cross platform build on GitHub
- [ ] build pagefind indices within single executable
- [ ] try [tinysearch](https://github.com/tinysearch/tinysearch) to index individual activities and link to within-page anchors?
- [ ] better CLI with progress bars and all that jazz
- [ ] produce a better set of export samples as test data
- [ ] also support .zip exports
