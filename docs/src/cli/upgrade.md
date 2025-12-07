# The `upgrade` command

The `upgrade` command is used to upgrade the database and perform any other
necessary changes after downloading a new version of Fossilizer.

Run this command whenever you upgrade Fossilizer.

```bash
cd my-mastodon-site
fossilizer upgrade
```

## Automatic Data Migration

Fossilizer automatically handles some data format migrations when reading from the database. For example, if you have data imported with an older version, the newer version will transparently upgrade the internal format when generating your site.

This means you generally don't need to re-import your data when upgrading Fossilizer - just run `upgrade` and rebuild your site.
