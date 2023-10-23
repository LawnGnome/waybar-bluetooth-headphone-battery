# waybar-bluetooth-headphone-battery

A [Waybar](https://github.com/Alexays/Waybar) custom module that will output the
battery status of certain kinds of Bluetooth device — by default, headphones and
headsets. If no matching devices are found, then nothing is displayed.

## Building

Standard Rust project, so:

```bash
cargo build --release
```

Will get you a binary in `target/release/waybar-bluetooth-headphone-battery`.

## Running

By default, this will run once and output a Waybar JSON blob with the current
battery status.

You can provide `--listen` and (optionally) a refresh frequency with
`--refresh`, and then this will run indefinitely, responding to upower events
and periodic refreshes. This is useful for Waybar! My configuration looks like
this:

```json
{
    "custom/headphone-battery": {
        "format": "{} {icon}",
        "format-icons": ["", "", "", "", ""],
        "exec": "$HOME/bin/waybar-bluetooth-headphone-battery --listen -r 10s",
        "return-type": "json"
    }
}
```

## Known issues

Multiple devices will probably be handled badly.
