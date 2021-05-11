# wlgreet

Raw wayland greeter for greetd, to be run under sway or similar. Note that cage is currently not supported due to it lacking wlr-layer-shell-unstable support.

See the [wiki](https://man.sr.ht/~kennylevinsen/greetd) for FAQ, guides for common configurations, and troubleshooting information.

![screenshot](https://github.com/loserMcloser/wlgreet-smallbox/blob/mainline/assets/screenshot.png)

## How to use

See the wiki.

## How to build

```
cargo build --release
cp target/release/wlgreet /usr/local/bin/
```

## How to discuss

Go to #kennylevinsen @ irc.libera.chat to discuss, or use [~kennylevinsen/greetd-devel@lists.sr.ht](https://lists.sr.ht/~kennylevinsen/greetd-devel).
