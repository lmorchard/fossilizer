# The `mastodon` sub-commands

The `mastodon` collection of sub-commands is used to connect to a Mastodon instance and fetch toots from an account via the Mastodon API.

To use these commands, first you'll need to connect to an existing account on a Mastodon instance using `link`, `code`, and then `verify` sub-commands.

Then, you can fetch toots from that account and import them into the local database using the `fetch` sub-command.

## Selecting a Mastodon instance

By default, the `mastodon` command will connect to the instance at `https://mastodon.social`. You can specify a different instance hostname with the `--instance` / `-i` option:

```bash
fossilizer mastodon --instance mstdn.social link
```

Configuration and secrets for connecting to the selected Mastodon instance are stored in a file named `config-{instance}.toml` in the `data` directory.

## Connecting to a Mastodon instance

Before importing toots from a Mastodon account, you'll need to connect to the instance and authenticate with an account.

The `link` sub-command will begin this process by attempting to register a new application with your instance and then offering an authorization URL to visit in a web browser. For example:

```bash
$ fossilizer mastodon link

[2024-04-18T20:06:21Z INFO  fossilizer::cli::mastodon::link] Visit this link to begin authorization:
[2024-04-18T20:06:21Z INFO  fossilizer::cli::mastodon::link] https://mastodon.social/oauth/authorize?client_id=w1pCC1ANqOqnrG6pk8cnbcMa0vTQjgmLQBHCrMqhEzY&scope=read+read%3Anotifications+read%3Astatuses+write+follow+push&redirect_uri=urn%3Aietf%3Awg%3Aoauth%3A2.0%3Aoob&response_type=code
```

Once you've visited this link and authorized the application, you'll be given a code to paste back into the terminal to complete the process.

The `code` sub-command will complete the process by exchanging the code for an access token:

```bash
$ fossilizer mastodon code 8675309jennyabcdefghiZZZFUVMixgjTlQMF0vK1I
```

After running the `code` sub-command, you can then run the `verify` sub-command to check that the connection is working:

```bash
$ fossilizer mastodon verify

[2024-04-18T20:09:04Z INFO  fossilizer::cli::mastodon::verify] Verified as AuthVerifyResult { username: "lmorchard", url: "https://mastodon.social/@lmorchard", display_name: "Les Orchard 🕹\u{fe0f}🔧🐱🐰", created_at: "2016-11-01T00:00:00.000Z" }
```

Note that the access token secret obtained through the above steps is stored in the `config-{instance}.toml` file in the `data` directory:

```
data
├── config-instance-hackers.town.toml
├── config-instance-mastodon.social.toml
└── data.sqlite3
```

Keep these files safe and don't publish them anywhere! Also, once you've connected to an instance, you can use the `--instance` / `-i` option to select it without needing to run `link` or `code` again.

## Fetching toots

Once you've connected to a Mastodon instance, you can import toots from an account with the `fetch` sub-command. By default, this command will attempt to fetch and import the newest 100 toots in pages of 25.

```bash
$ fossilizer mastodon fetch

[2024-04-18T20:13:00Z INFO  fossilizer::mastodon::fetcher] Fetching statuses for account https://mastodon.social/@lmorchard
[2024-04-18T20:13:01Z INFO  fossilizer::mastodon::fetcher] Fetched 25 (of 100 max)...
[2024-04-18T20:13:04Z INFO  fossilizer::mastodon::fetcher] Fetched 50 (of 100 max)...
[2024-04-18T20:13:04Z INFO  fossilizer::mastodon::fetcher] Fetched 75 (of 100 max)...
[2024-04-18T20:13:05Z INFO  fossilizer::mastodon::fetcher] Fetched 100 (of 100 max)...
```

You can adjust the number of toots fetched with the `--max` / `-m` option and the page size with the `--page` / `-p` option. However, note that the Mastodon API may limit the number of toots you can fetch in a single request:

```bash
$ fossilizer mastodon fetch --max 200 --page 100

[2024-04-18T20:15:28Z INFO  fossilizer::mastodon::fetcher] Fetching statuses for account https://mastodon.social/@lmorchard
[2024-04-18T20:15:29Z INFO  fossilizer::mastodon::fetcher] Fetched 40 (of 200 max)...
[2024-04-18T20:15:29Z INFO  fossilizer::mastodon::fetcher] Fetched 80 (of 200 max)...
[2024-04-18T20:15:30Z INFO  fossilizer::mastodon::fetcher] Fetched 120 (of 200 max)...
[2024-04-18T20:15:31Z INFO  fossilizer::mastodon::fetcher] Fetched 160 (of 200 max)...
[2024-04-18T20:15:31Z INFO  fossilizer::mastodon::fetcher] Fetched 200 (of 200 max)...
```

### Incremental fetching

If you've already imported most of your toots and would like to fetch only the newest ones, you can use the `--incremental` option. This will stop the fetch process as soon as a page is encountered that contains a toot already in the database:

```bash
$ fossilizer mastodon fetch --incremental

2024-04-18T20:17:49Z INFO  fossilizer::mastodon::fetcher] Fetching statuses for account https://mastodon.social/@lmorchard
[2024-04-18T20:17:50Z INFO  fossilizer::mastodon::fetcher] Fetched 25 (of 100 max)...
[2024-04-18T20:17:50Z INFO  fossilizer::mastodon::fetcher] Stopping incremental fetch after catching up to imported activities
```
