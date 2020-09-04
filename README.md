# Pertinent - Get To The Point!

This is a learning-exercise rewrite of [Pinpoint](http://wiki.gnome.org/Apps/Pinpoint) in [Rust](https://www.rust-lang.org/). The upstream Pinpoint project appears to be abandoned and started to segfault for me. The most recently active git repo I can find is [ffrank/gnome-pinpoint](https://github.com/ffrank/gnome-pinpoint). I've started to use [reveal-md](https://github.com/webpro/reveal-md) for my actual presentations, but this is fun anyway :)

## Presentation File Format

The slide-deck has an initial settings section and then slides with a simple `-` separator. Each slide can have additional settings. The content of the slide is generally "pango" and is centered and big on the screen.

Example:

```
# the 0th "slide" provides default styling for the presentation
[bottom]           # position of text
[slide-bg.jpg]     # default slide background
--- [black] [center] # override background and text position

A presentation

--------- # lines starting with hyphens separate slides

The format is meant to be <u>simple</u>

--- [ammo.jpg]  # override background

• Bullet point lists through unicode
• Evil, but sometimes needed
```

