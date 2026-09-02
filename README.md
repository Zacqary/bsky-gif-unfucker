# Bluesky Gif Unfucker

This is a tool for artists and animators that unfucks your gifs before you upload them to Bluesky.

## Why

Bluesky's gif to video converter can't output a video that's less than 1 second long. If you upload a looping animation that's less than 1 second long, the video converter will freeze the final frame for as long as it needs to reach that 1 second minimum. This absolutely fucks your carefully crafted animation loop.

There are two possible solutions to this:

### 1. Report this bug to Bluesky and hope they fix it

Ha. Ha ha. Ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha ha. Ha.

### 2. Duplicate your gif's frames until it's at least 1 second long

That's what Bluesky Gif Unfucker does for you so you don't have to!

## Usage

### Download for macOS

You can download the latest bundled macOS app from the [releases page](https://github.com/Zacqary/bsky-gif-unfucker/release). I haven't packaged it for Windows or Linux yet cause I just threw this shit together one night. Yell at me if you want this. Idk I'll do it eventually.

### Build from source

Check out this code, [install Rust](https://rust-lang.org/tools/install/) and then

```sh
cargo run
```

This probably works on Windows and Linux. Idk.
