# The `import` command

The `import` command is used to ingest the content from a Mastodon export into
the SQLite database and extract media attachments. It's used like so:

```bash
cd my-mastodon-site
fossilizer import ../archive-20230720182703-36f08a7ce74bbf59f141b496b2b7f457.tar.gz
```

Depending on the size of your export, this command should take a few seconds or
minutes to extract all the posts and attachments.

Along with inserting database records, you'll find files like the following
added to your data directory, including all the media attachments associated
with the export under a directory based on the SHA-256 hash of the account
address:

```bash
my-mastodon-site/
└── data
    ├── data.sqlite3
    ├── media
    │   └── acc0bb231a7a2757c7e5c63aa68ce3cdbcfd32a43eb67a6bdedffe173c721184
    │       ├── avatar.png
    │       ├── header.jpg
    │       └── media_attachments
    │           └── files
    │               ├── 002
    │               │   ├── ...
    │               ├── 105
    │               │   ├── ...
    │               ├── 106
    │               │   ├── ...
```

You can run this command repeatedly, either with fresh exports from one
Mastodon instance or with exports from many instances. All the data will be
merged into the database from previous imports.

After you've run this command, you can try [the `build` command](./build.md) to
generate a static web site.
