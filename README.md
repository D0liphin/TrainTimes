# `lcdterm`

Use any RGB565 LCD as a terminal! 

![The terminal in use](image/IMG_TERM_IN_USE.jpg)

I might make this a full GUI library, but for now it's just supposed
to be a terminal (**not** ANSI). We want:

- [x] Lazily write ascii to any location on the terminal, and then flush
  to display...
  - [ ] as efficiently as possible (prepare new window only when needed)
- [ ] Background and foreground colors for all letters
- [ ] Scrollable sections -- mark a region as scrollable in the x or y
  direction and have it scroll pixel by pixel*
- [x] Easy interface to drivers, for basically any rgb565 display.
  - [x] With out of the box driver for ST7789
- [ ] low memory footprint

*I only plan for this to be added as a "scroll by a char" option. This
means that the terminal should always be aligned to a char boundary, and
half-chars are only displayed during the scrolling step.